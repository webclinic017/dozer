use crate::pipeline::errors::ProductError;
use crate::pipeline::product::set::{SetAction, SetOperation};
use dozer_core::channels::ProcessorChannelForwarder;
use dozer_core::epoch::Epoch;
use dozer_core::errors::ExecutionError;
use dozer_core::node::{PortHandle, Processor};
use dozer_core::record_store::RecordReader;
use dozer_core::storage::lmdb_storage::SharedTransaction;
use dozer_core::DEFAULT_PORT_HANDLE;
use dozer_types::types::{Operation, Record};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SetProcessor {
    /// Set operations
    operator: SetOperation,
    record_hash_map: Vec<u64>,
}

impl SetProcessor {
    /// Creates a new [`FromProcessor`].
    pub fn new(operator: SetOperation) -> Self {
        Self {
            operator,
            record_hash_map: Vec::new(),
        }
    }

    fn delete(
        &mut self,
        from_port: PortHandle,
        record: &Record,
    ) -> Result<Vec<(SetAction, Record)>, ProductError> {
        self.operator
            .execute(
                SetAction::Delete,
                from_port,
                record,
                &mut self.record_hash_map,
            )
            .map_err(|err| {
                ProductError::DeleteError("UNION query error:".to_string(), Box::new(err))
            })
    }

    fn insert(
        &mut self,
        from_port: PortHandle,
        record: &Record,
    ) -> Result<Vec<(SetAction, Record)>, ProductError> {
        self.operator
            .execute(
                SetAction::Insert,
                from_port,
                record,
                &mut self.record_hash_map,
            )
            .map_err(|err| {
                ProductError::InsertError("UNION query error:".to_string(), Box::new(err))
            })
    }

    #[allow(clippy::type_complexity)]
    fn update(
        &mut self,
        from_port: PortHandle,
        old: &Record,
        new: &Record,
    ) -> Result<(Vec<(SetAction, Record)>, Vec<(SetAction, Record)>), ProductError> {
        let old_records = self
            .operator
            .execute(SetAction::Delete, from_port, old, &mut self.record_hash_map)
            .map_err(|err| {
                ProductError::UpdateOldError("UNION query error:".to_string(), Box::new(err))
            })?;

        let new_records = self
            .operator
            .execute(SetAction::Insert, from_port, new, &mut self.record_hash_map)
            .map_err(|err| {
                ProductError::UpdateNewError("UNION query error:".to_string(), Box::new(err))
            })?;

        Ok((old_records, new_records))
    }
}

impl Processor for SetProcessor {
    fn commit(&self, _epoch: &Epoch, _tx: &SharedTransaction) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn process(
        &mut self,
        from_port: PortHandle,
        op: Operation,
        fw: &mut dyn ProcessorChannelForwarder,
        _transaction: &SharedTransaction,
        _reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<(), ExecutionError> {
        match op {
            Operation::Delete { ref old } => {
                let records = self
                    .delete(from_port, old)
                    .map_err(|err| ExecutionError::ProductProcessorError(Box::new(err)))?;

                for (action, record) in records.into_iter() {
                    match action {
                        SetAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: record }, DEFAULT_PORT_HANDLE);
                        }
                        SetAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: record }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
            Operation::Insert { ref new } => {
                let records = self
                    .insert(from_port, new)
                    .map_err(|err| ExecutionError::ProductProcessorError(Box::new(err)))?;

                for (action, record) in records.into_iter() {
                    match action {
                        SetAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: record }, DEFAULT_PORT_HANDLE);
                        }
                        SetAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: record }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
            Operation::Update { ref old, ref new } => {
                let (old_join_records, new_join_records) = self
                    .update(from_port, old, new)
                    .map_err(|err| ExecutionError::ProductProcessorError(Box::new(err)))?;

                for (action, old) in old_join_records.into_iter() {
                    match action {
                        SetAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: old }, DEFAULT_PORT_HANDLE);
                        }
                        SetAction::Delete => {
                            let _ = fw.send(Operation::Delete { old }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }

                for (action, new) in new_join_records.into_iter() {
                    match action {
                        SetAction::Insert => {
                            let _ = fw.send(Operation::Insert { new }, DEFAULT_PORT_HANDLE);
                        }
                        SetAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: new }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}