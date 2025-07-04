//! The module contains implementations and tests for the `ContractsState` table.

#[cfg(feature = "smt")]
mod smt {
    use crate::{
        Mappable,
        blueprint::sparse::{
            PrimaryKey,
            Sparse,
        },
        codec::raw::Raw,
        column::Column,
        structured_storage::TableWithBlueprint,
        tables::{
            ContractsState,
            merkle::{
                ContractsStateMerkleData,
                ContractsStateMerkleMetadata,
            },
        },
    };
    use alloc::borrow::Cow;

    /// The key converter used to convert the key from the `ContractsState` table
    /// to the key of the `ContractsStateMerkleMetadata` table.
    pub struct KeyConverter;

    impl PrimaryKey for KeyConverter {
        type InputKey = <ContractsState as Mappable>::Key;
        type OutputKey = <ContractsStateMerkleMetadata as Mappable>::Key;

        fn primary_key(key: &Self::InputKey) -> Cow<Self::OutputKey> {
            Cow::Borrowed(key.contract_id())
        }
    }

    impl TableWithBlueprint for ContractsState {
        type Blueprint = Sparse<
            Raw,
            Raw,
            ContractsStateMerkleMetadata,
            ContractsStateMerkleData,
            KeyConverter,
        >;
        type Column = Column;

        fn column() -> Column {
            Column::ContractsState
        }
    }

    #[cfg(test)]
    #[allow(non_snake_case)]
    mod test {
        use rand::{
            Rng,
            prelude::StdRng,
        };

        use crate::blueprint::sparse::root_storage_tests_smt::SMTTestDataGenerator;

        use super::*;

        fn generate_key(
            primary_key: &<ContractsStateMerkleMetadata as Mappable>::Key,
            rng: &mut impl rand::Rng,
        ) -> <ContractsState as Mappable>::Key {
            let mut bytes = [0u8; 32];
            rng.fill(bytes.as_mut());
            <ContractsState as Mappable>::Key::new(primary_key, &bytes.into())
        }

        fn generate_key_for_same_contract(
            rng: &mut impl rand::Rng,
        ) -> <ContractsState as Mappable>::Key {
            generate_key(&fuel_core_types::fuel_tx::ContractId::zeroed(), rng)
        }

        crate::basic_storage_tests!(
            ContractsState,
            <ContractsState as Mappable>::Key::default(),
            [0u8; 32],
            vec![0u8; 32].into(),
            generate_key_for_same_contract
        );

        impl SMTTestDataGenerator for ContractsState {
            type Key = <ContractsState as Mappable>::Key;
            type PrimaryKey = <ContractsStateMerkleMetadata as Mappable>::Key;
            type Value = <ContractsState as Mappable>::OwnedValue;

            fn primary_key() -> Self::PrimaryKey {
                fuel_core_types::fuel_tx::ContractId::from([1u8; 32])
            }

            fn foreign_key() -> Self::PrimaryKey {
                fuel_core_types::fuel_tx::ContractId::from([2u8; 32])
            }

            fn generate_key(
                current_key: &Self::PrimaryKey,
                rng: &mut StdRng,
            ) -> Self::Key {
                let mut bytes = [0u8; 32];
                rng.fill(bytes.as_mut());
                <ContractsState as Mappable>::Key::new(current_key, &bytes.into())
            }

            fn generate_value(rng: &mut StdRng) -> Self::Value {
                let mut bytes = [0u8; 32];
                rng.fill(bytes.as_mut());
                <ContractsState as Mappable>::OwnedValue::from(bytes.as_slice())
            }
        }

        crate::root_storage_tests!(ContractsState);
    }

    #[cfg(test)]
    #[allow(non_snake_case)]
    mod structured_storage_tests {
        use crate::{
            StorageAsMut,
            StorageMutate,
            StorageWrite,
            column::Column,
            structured_storage::test::InMemoryStorage,
            transactional::ReadTransaction,
        };
        use fuel_vm_private::{
            prelude::{
                Bytes32,
                ContractId,
            },
            storage::{
                ContractsState,
                ContractsStateKey,
            },
        };
        use rand::{
            Rng,
            SeedableRng,
            prelude::StdRng,
        };

        #[test]
        fn storage_write__write__generates_the_same_merkle_root_as_storage_insert() {
            type Storage = InMemoryStorage<Column>;

            let mut rng = StdRng::seed_from_u64(1234);

            // Given
            let contract_id = ContractId::default();
            let keys = std::iter::from_fn(|| Some(rng.r#gen::<Bytes32>()))
                .take(10)
                .map(|state_key| ContractsStateKey::from((&contract_id, &state_key)))
                .collect::<Vec<_>>();
            let value = vec![0u8; 32];

            // When
            let merkle_root_write = {
                let storage = Storage::default();
                let mut structure = storage.read_transaction();
                let mut merkle_root = structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root");
                for key in keys.iter() {
                    <_ as StorageWrite<ContractsState>>::write_bytes(
                        &mut structure,
                        key,
                        &value,
                    )
                    .expect("Unable to write storage");
                    let new_merkle_root = structure
                        .storage::<ContractsState>()
                        .root(&contract_id)
                        .expect("Unable to retrieve Merkle root");
                    assert_ne!(merkle_root, new_merkle_root);
                    merkle_root = new_merkle_root;
                }

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            // Then
            let merkle_root_insert = {
                let storage = Storage::default();
                let mut structure = storage.read_transaction();
                for key in keys.iter() {
                    <_ as StorageMutate<ContractsState>>::insert(
                        &mut structure,
                        key,
                        &value,
                    )
                    .expect("Unable to write storage");
                }

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            assert_eq!(merkle_root_write, merkle_root_insert);
        }

        #[test]
        fn storage_write__replace__generates_the_same_merkle_root_as_storage_insert() {
            type Storage = InMemoryStorage<Column>;

            let mut rng = StdRng::seed_from_u64(1234);

            // Given
            let contract_id = ContractId::default();
            let keys = std::iter::from_fn(|| Some(rng.r#gen::<Bytes32>()))
                .take(10)
                .map(|state_key| ContractsStateKey::from((&contract_id, &state_key)))
                .collect::<Vec<_>>();
            let value = vec![0u8; 32];

            // When
            let merkle_root_replace = {
                let storage = Storage::default();
                let mut structure = storage.read_transaction();
                let mut merkle_root = structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root");
                for key in keys.iter() {
                    <_ as StorageWrite<ContractsState>>::replace_bytes(
                        &mut structure,
                        key,
                        &value,
                    )
                    .expect("Unable to write storage");
                    let new_merkle_root = structure
                        .storage::<ContractsState>()
                        .root(&contract_id)
                        .expect("Unable to retrieve Merkle root");
                    assert_ne!(merkle_root, new_merkle_root);
                    merkle_root = new_merkle_root;
                }

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            // Then
            let merkle_root_insert = {
                let storage = Storage::default();
                let mut structure = storage.read_transaction();
                for key in keys.iter() {
                    <_ as StorageMutate<ContractsState>>::insert(
                        &mut structure,
                        key,
                        &value,
                    )
                    .expect("Unable to write storage");
                }

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            assert_eq!(merkle_root_replace, merkle_root_insert);
        }

        #[test]
        fn storage_write__take__generates_the_same_merkle_root_as_storage_remove() {
            type Storage = InMemoryStorage<Column>;

            let mut rng = StdRng::seed_from_u64(1234);

            // Given
            let contract_id = ContractId::default();
            let keys = std::iter::from_fn(|| Some(rng.r#gen::<Bytes32>()))
                .take(10)
                .map(|state_key| ContractsStateKey::from((&contract_id, &state_key)))
                .collect::<Vec<_>>();
            let value = vec![0u8; 32];

            let storage = Storage::default();
            let mut structure = storage.read_transaction();
            let mut merkle_root = structure
                .storage::<ContractsState>()
                .root(&contract_id)
                .expect("Unable to retrieve Merkle root");
            for key in keys.iter() {
                <_ as StorageWrite<ContractsState>>::replace_bytes(
                    &mut structure,
                    key,
                    &value,
                )
                .expect("Unable to write storage");

                let new_merkle_root = structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root");
                assert_ne!(merkle_root, new_merkle_root);
                merkle_root = new_merkle_root;
            }

            // When
            let state_key = rng.r#gen::<Bytes32>();
            let key = ContractsStateKey::from((&contract_id, &state_key));

            let merkle_root_replace = {
                <_ as StorageWrite<ContractsState>>::write_bytes(
                    &mut structure,
                    &key,
                    &value,
                )
                .expect("Unable to write storage");

                <_ as StorageWrite<ContractsState>>::take_bytes(&mut structure, &key)
                    .expect("Unable to take value from storage");

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            // Then
            let merkle_root_remove = {
                <_ as StorageWrite<ContractsState>>::write_bytes(
                    &mut structure,
                    &key,
                    &value,
                )
                .expect("Unable to write storage");

                structure
                    .storage::<ContractsState>()
                    .remove(&key)
                    .expect("Unable to take value from storage");

                structure
                    .storage::<ContractsState>()
                    .root(&contract_id)
                    .expect("Unable to retrieve Merkle root")
            };

            assert_eq!(merkle_root_replace, merkle_root_remove);
        }
    }
}

#[cfg(not(feature = "smt"))]
mod plain {
    use crate::{
        blueprint::plain::Plain,
        codec::raw::Raw,
        column::Column,
        structured_storage::TableWithBlueprint,
        tables::ContractsState,
    };

    impl TableWithBlueprint for ContractsState {
        type Blueprint = Plain<Raw, Raw>;
        type Column = Column;

        fn column() -> Column {
            Column::ContractsState
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use crate::Mappable;
        use fuel_core_types::fuel_tx::ContractId;

        fn generate_key(
            primary_key: &ContractId,
            rng: &mut impl rand::Rng,
        ) -> <ContractsState as Mappable>::Key {
            let mut bytes = [0u8; 32];
            rng.fill(bytes.as_mut());
            <ContractsState as Mappable>::Key::new(primary_key, &bytes.into())
        }

        fn generate_key_for_same_contract(
            rng: &mut impl rand::Rng,
        ) -> <ContractsState as Mappable>::Key {
            generate_key(&fuel_core_types::fuel_tx::ContractId::zeroed(), rng)
        }

        crate::basic_storage_tests!(
            ContractsState,
            <ContractsState as Mappable>::Key::default(),
            [0u8; 32],
            vec![0u8; 32].into(),
            generate_key_for_same_contract
        );
    }
}
