use std::collections::HashMap;

use heed::types::{SerdeBincode, Str};
use heed::Env;

use crate::{errors::RustlinksError, rustlink::Rustlink};

pub struct LMDB {
    db_env: Env,
}

impl LMDB {
    pub fn new(db_env: Env) -> Self {
        Self { db_env }
    }
}

impl RustlinkStore for LMDB {
    fn get_rustlink(&self, key: &str) -> Result<Option<Rustlink>, RustlinksError> {
        let rtxn = self.db_env.read_txn()?;
        let rustlink = if let Some(db) = self
            .db_env
            .open_database::<Str, SerdeBincode<Rustlink>>(&rtxn, None)?
        {
            db.get(&rtxn, &key)?
        } else {
            None
        };
        rtxn.commit()?;
        Ok(rustlink)
    }

    fn set_rustlink(&self, key: &str, value: &Rustlink) -> Result<(), RustlinksError> {
        let mut wtxn = self.db_env.write_txn()?;
        let db = self
            .db_env
            .create_database::<Str, SerdeBincode<Rustlink>>(&mut wtxn, None)?;
        db.put(&mut wtxn, &key, &value)?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete_rustlink(&self, key: &str) -> Result<(), RustlinksError> {
        let mut wtxn = self.db_env.write_txn()?;
        if let Some(db) = self
            .db_env
            .open_database::<Str, SerdeBincode<Rustlink>>(&mut wtxn, None)?
        {
            db.delete(&mut wtxn, &key)?;
        }
        wtxn.commit()?;
        Ok(())
    }

    fn list_rustlinks(&self) -> Result<Vec<Rustlink>, RustlinksError> {
        let rtxn = self.db_env.read_txn()?;
        let mut rustlinks = vec![];
        if let Some(db) = self
            .db_env
            .open_database::<Str, SerdeBincode<Rustlink>>(&rtxn, None)?
        {
            rustlinks = db
                .iter(&rtxn)?
                .filter_map(|f| {
                    f.ok().map(|(key, mut v)| {
                        v.name = Some(key.to_string());
                        v
                    })
                })
                .collect::<Vec<Rustlink>>();
        }
        rtxn.commit()?;
        Ok(rustlinks)
    }

    fn get_revision(&self) -> Result<i64, RustlinksError> {
        let rtxn = self.db_env.read_txn()?;
        let revision = if let Some(db) = self
            .db_env
            .open_database::<Str, SerdeBincode<i64>>(&rtxn, Some("revisions"))?
        {
            db.get(&rtxn, &"revision")?.unwrap_or(0)
        } else {
            0
        };
        rtxn.commit()?;
        Ok(revision)
    }

    fn set_revision(&self, revision: i64) -> Result<(), RustlinksError> {
        let mut wtxn = self.db_env.write_txn()?;
        let db = self
            .db_env
            .create_database::<Str, SerdeBincode<i64>>(&mut wtxn, Some("revisions"))?;
        db.put(&mut wtxn, &"revision", &revision)?;
        wtxn.commit()?;
        Ok(())
    }

    fn set_rustlinks(&self, rustlinks: HashMap<&str, Rustlink>) -> Result<(), RustlinksError> {
        let mut wtxn = self.db_env.write_txn()?;
        let db = self
            .db_env
            .create_database::<Str, SerdeBincode<Rustlink>>(&mut wtxn, None)?;
        for (key, value) in rustlinks {
            db.put(&mut wtxn, &key, &value)?;
        }
        wtxn.commit()?;
        Ok(())
    }
}

pub trait RustlinkStore {
    fn get_rustlink(&self, key: &str) -> Result<Option<Rustlink>, RustlinksError>;
    fn set_rustlink(&self, key: &str, value: &Rustlink) -> Result<(), RustlinksError>;
    fn set_rustlinks(&self, rustlinks: HashMap<&str, Rustlink>) -> Result<(), RustlinksError>;
    fn delete_rustlink(&self, key: &str) -> Result<(), RustlinksError>;
    fn list_rustlinks(&self) -> Result<Vec<Rustlink>, RustlinksError>;
    fn get_revision(&self) -> Result<i64, RustlinksError>;
    fn set_revision(&self, revision: i64) -> Result<(), RustlinksError>;
}

#[cfg(test)]
mod tests {
    use heed::EnvOpenOptions;

    use super::*;
    use crate::rustlink::RustlinkType::LinkedIn;

    #[test]
    fn test_lmdb_rustlink_rw() {
        let dir = tempfile::tempdir().unwrap();
        let env = EnvOpenOptions::new()
            .map_size(10_485_760)
            .max_dbs(2)
            .open(&dir.path())
            .unwrap();
        let store = LMDB::new(env);
        let rustlink = Rustlink::new("https://example.com".to_string(), LinkedIn, 0);

        store.set_rustlink("example", &rustlink).unwrap();
        let result = store.get_rustlink("example").unwrap();
        assert_eq!(result, Some(rustlink.clone()));

        store.delete_rustlink("example").unwrap();
        let result = store.get_rustlink("example").unwrap();
        assert_eq!(result, None);

        store.set_rustlink("example", &rustlink).unwrap();
        store.set_rustlink("example2", &rustlink).unwrap();
        let result = store.list_rustlinks().unwrap();
        assert_eq!(result.len(), 2);

        store.set_revision(123).unwrap();
        let revision = store.get_revision().unwrap();
        assert_eq!(revision, 123);
    }
}
