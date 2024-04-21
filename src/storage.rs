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
    fn get(&self, key: &str) -> Result<Option<Rustlink>, RustlinksError> {
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

    fn set(&self, key: &str, value: &Rustlink) -> Result<(), RustlinksError> {
        let mut wtxn = self.db_env.write_txn()?;
        let db = self
            .db_env
            .create_database::<Str, SerdeBincode<Rustlink>>(&mut wtxn, None)?;
        db.put(&mut wtxn, &key, &value)?;
        wtxn.commit()?;
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<(), RustlinksError> {
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

    fn list(&self) -> Result<Vec<Rustlink>, RustlinksError> {
        let rtxn = self.db_env.read_txn()?;
        let mut rustlinks = vec![];
        if let Some(db) = self
            .db_env
            .open_database::<Str, SerdeBincode<Rustlink>>(&rtxn, None)?
        {
            rustlinks = db
                .iter(&rtxn)?
                .filter_map(|f| f.ok().map(|(_, v)| v))
                .collect::<Vec<Rustlink>>();
        }
        rtxn.commit()?;
        Ok(rustlinks)
    }
}

pub trait RustlinkStore {
    fn get(&self, key: &str) -> Result<Option<Rustlink>, RustlinksError>;
    fn set(&self, key: &str, value: &Rustlink) -> Result<(), RustlinksError>;
    fn delete(&self, key: &str) -> Result<(), RustlinksError>;
    fn list(&self) -> Result<Vec<Rustlink>, RustlinksError>;
}

#[cfg(test)]
mod tests {
    use heed::EnvOpenOptions;

    use super::*;

    #[test]
    fn test_lmdb() {
        let dir = tempfile::tempdir().unwrap();
        let env = EnvOpenOptions::new()
            .map_size(10_485_760)
            .open(&dir.path())
            .unwrap();
        let store = LMDB::new(env);

        let rustlink = Rustlink {
            url: "https://example.com".to_string(),
            _type: crate::rustlink::RustlinkType::LinkedIn,
            revision: 0,
        };

        store.set("example", &rustlink).unwrap();
        let result = store.get("example").unwrap();
        assert_eq!(result, Some(rustlink));

        store.delete("example").unwrap();
        let result = store.get("example").unwrap();
        assert_eq!(result, None);

        dir.close().unwrap();
    }
}
