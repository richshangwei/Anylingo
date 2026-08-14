use std::path::Path;

use rusqlite::{Connection, params};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    Anthropic,
    AzureOpenAi,
    GoogleGemini,
    OpenAiCompatible,
    OpenRouter,
    XAi,
    OllamaNative,
    FedGpt,
    CustomEndpoint,
}

impl ProviderKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::AzureOpenAi => "azure-openai",
            Self::GoogleGemini => "google-gemini",
            Self::OpenAiCompatible => "openai-compatible",
            Self::OpenRouter => "openrouter",
            Self::XAi => "xai",
            Self::OllamaNative => "ollama-native",
            Self::FedGpt => "fedgpt",
            Self::CustomEndpoint => "custom-endpoint",
        }
    }

    fn parse(value: &str) -> Result<Self, StorageError> {
        match value {
            "anthropic" => Ok(Self::Anthropic),
            "azure-openai" => Ok(Self::AzureOpenAi),
            "google-gemini" => Ok(Self::GoogleGemini),
            "openai-compatible" => Ok(Self::OpenAiCompatible),
            "openrouter" => Ok(Self::OpenRouter),
            "xai" => Ok(Self::XAi),
            "ollama-native" => Ok(Self::OllamaNative),
            "fedgpt" => Ok(Self::FedGpt),
            "custom-endpoint" => Ok(Self::CustomEndpoint),
            other => Err(StorageError::UnknownProvider(other.to_owned())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub provider: ProviderKind,
    pub endpoint: String,
    pub model: String,
    pub credential_key: Option<String>,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("unknown provider kind: {0}")]
    UnknownProvider(String),
    #[error(transparent)]
    Secret(#[from] SecretStoreError),
}

#[derive(Debug, Error)]
#[error("credential operation failed: {message}")]
pub struct SecretStoreError {
    message: String,
}

impl SecretStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait SecretStore {
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;
    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError>;
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
}

#[cfg(target_os = "windows")]
pub struct WindowsCredentialStore {
    service: String,
}

#[cfg(target_os = "windows")]
impl WindowsCredentialStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(&self.service, key)
            .map_err(|error| SecretStoreError::new(error.to_string()))
    }
}

#[cfg(target_os = "windows")]
impl SecretStore for WindowsCredentialStore {
    fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        self.entry(key)?
            .set_password(value)
            .map_err(|error| SecretStoreError::new(error.to_string()))
    }

    fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(SecretStoreError::new(error.to_string())),
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretStoreError::new(error.to_string())),
        }
    }
}

pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS model_profiles (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                model TEXT NOT NULL,
                credential_key TEXT
            );
            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(Self { connection })
    }

    /// 偏好設定以字串鍵值存放。刻意不放進 model_profiles：
    /// 那張表是「模型連線設定」，介面偏好與它無關，混在一起會讓兩者互相牽制。
    pub fn preference(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT value FROM preferences WHERE key = ?1")?;
        let mut rows = statement.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub fn set_preference(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "
            INSERT INTO preferences (key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn save_model_profile(&mut self, profile: &ModelProfile) -> Result<(), StorageError> {
        self.connection.execute(
            "
            INSERT INTO model_profiles (id, name, provider, endpoint, model, credential_key)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                provider = excluded.provider,
                endpoint = excluded.endpoint,
                model = excluded.model,
                credential_key = excluded.credential_key
            ",
            params![
                profile.id,
                profile.name,
                profile.provider.as_str(),
                profile.endpoint,
                profile.model,
                profile.credential_key,
            ],
        )?;
        Ok(())
    }

    pub fn model_profiles(&self) -> Result<Vec<ModelProfile>, StorageError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, name, provider, endpoint, model, credential_key
            FROM model_profiles
            ORDER BY name COLLATE NOCASE, id
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;

        rows.map(|row| {
            let (id, name, provider, endpoint, model, credential_key) = row?;
            Ok(ModelProfile {
                id,
                name,
                provider: ProviderKind::parse(&provider)?,
                endpoint,
                model,
                credential_key,
            })
        })
        .collect()
    }

    pub fn model_profile(&self, id: &str) -> Result<Option<ModelProfile>, StorageError> {
        let mut profiles = self.model_profiles()?;
        Ok(profiles.drain(..).find(|profile| profile.id == id))
    }
}

pub struct LocalData<S> {
    database: SqliteStore,
    secrets: S,
}

impl<S> LocalData<S>
where
    S: SecretStore,
{
    pub fn new(database: SqliteStore, secrets: S) -> Self {
        Self { database, secrets }
    }

    pub fn save_model_profile(
        &mut self,
        profile: &ModelProfile,
        secret: Option<&str>,
    ) -> Result<(), StorageError> {
        if let (Some(key), Some(secret)) = (&profile.credential_key, secret) {
            self.secrets.set(key, secret)?;
        }

        if let Err(error) = self.database.save_model_profile(profile) {
            if let (Some(key), Some(_)) = (&profile.credential_key, secret) {
                let _ = self.secrets.delete(key);
            }
            return Err(error);
        }

        Ok(())
    }

    pub fn model_profiles(&self) -> Result<Vec<ModelProfile>, StorageError> {
        self.database.model_profiles()
    }

    pub fn model_profile(&self, id: &str) -> Result<Option<ModelProfile>, StorageError> {
        self.database.model_profile(id)
    }

    pub fn model_secret(&self, credential_key: &str) -> Result<Option<String>, StorageError> {
        Ok(self.secrets.get(credential_key)?)
    }

    /// 讀取布林偏好；沒設定過就用預設值，讓新加的偏好不必寫資料庫遷移。
    pub fn flag(&self, key: &str, default: bool) -> Result<bool, StorageError> {
        Ok(self
            .database
            .preference(key)?
            .map(|value| value == "true")
            .unwrap_or(default))
    }

    pub fn set_flag(&mut self, key: &str, value: bool) -> Result<(), StorageError> {
        self.database
            .set_preference(key, if value { "true" } else { "false" })
    }

    /// 讀取字串偏好。用於選項多於兩個、布林裝不下的設定（例如圖片辨識方式）。
    pub fn choice(&self, key: &str, default: &str) -> Result<String, StorageError> {
        Ok(self
            .database
            .preference(key)?
            .unwrap_or_else(|| default.to_owned()))
    }

    pub fn set_choice(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.database.set_preference(key, value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct TestSecrets(Mutex<HashMap<String, String>>);

    impl SecretStore for TestSecrets {
        fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
            self.0.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        }

        fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }

        fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn unset_preference_falls_back_to_the_supplied_default() {
        let directory = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(directory.path().join("floatrans.db")).unwrap();
        let mut data = LocalData::new(store, TestSecrets::default());

        // 沒寫過的鍵要回預設，新偏好才不必寫資料庫遷移
        assert!(data.flag("panel/auto-collapse", true).unwrap());
        assert!(!data.flag("panel/show-source", false).unwrap());

        data.set_flag("panel/show-source", true).unwrap();
        assert!(data.flag("panel/show-source", false).unwrap());

        data.set_flag("panel/show-source", false).unwrap();
        assert!(!data.flag("panel/show-source", true).unwrap());
    }

    #[test]
    fn saved_model_profile_is_retrievable_without_secret_material() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("floatrans.db");
        let mut store = SqliteStore::open(database).unwrap();
        let profile = ModelProfile {
            id: "local-ollama".into(),
            name: "本機 Ollama".into(),
            provider: ProviderKind::OllamaNative,
            endpoint: "http://127.0.0.1:11434".into(),
            model: "qwen3:8b".into(),
            credential_key: None,
        };

        store.save_model_profile(&profile).unwrap();

        assert_eq!(store.model_profiles().unwrap(), vec![profile]);
    }

    #[test]
    fn profile_secret_is_retrievable_only_through_the_secret_store() {
        let directory = tempfile::tempdir().unwrap();
        let database = SqliteStore::open(directory.path().join("floatrans.db")).unwrap();
        let mut data = LocalData::new(database, TestSecrets::default());
        let profile = ModelProfile {
            id: "cloud".into(),
            name: "OpenAI".into(),
            provider: ProviderKind::OpenAiCompatible,
            endpoint: "https://api.openai.com".into(),
            model: "gpt-5-mini".into(),
            credential_key: Some("profile/cloud/api-key".into()),
        };

        data.save_model_profile(&profile, Some("top-secret"))
            .unwrap();

        assert_eq!(data.model_profiles().unwrap(), vec![profile]);
        assert_eq!(
            data.model_secret("profile/cloud/api-key").unwrap(),
            Some("top-secret".into())
        );
    }

    #[test]
    fn model_profile_can_be_selected_by_stable_id() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = SqliteStore::open(directory.path().join("floatrans.db")).unwrap();
        let profile = ModelProfile {
            id: "preferred".into(),
            name: "Preferred model".into(),
            provider: ProviderKind::OllamaNative,
            endpoint: "http://127.0.0.1:11434".into(),
            model: "qwen3:8b".into(),
            credential_key: None,
        };
        store.save_model_profile(&profile).unwrap();

        assert_eq!(store.model_profile("preferred").unwrap(), Some(profile));
        assert_eq!(store.model_profile("missing").unwrap(), None);
    }

    #[test]
    fn internal_api_profile_round_trips_as_a_first_class_provider() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = SqliteStore::open(directory.path().join("floatrans.db")).unwrap();
        let profile = ModelProfile {
            id: "fedgpt".into(),
            name: "公司內部 API".into(),
            provider: ProviderKind::FedGpt,
            endpoint: "https://internal.example.com".into(),
            model: "translator".into(),
            credential_key: Some("profile/fedgpt/api-key".into()),
        };
        store.save_model_profile(&profile).unwrap();

        assert_eq!(store.model_profile("fedgpt").unwrap(), Some(profile));
    }

    #[test]
    fn every_provider_kind_round_trips_through_its_stable_storage_id() {
        let providers = [
            ProviderKind::Anthropic,
            ProviderKind::AzureOpenAi,
            ProviderKind::GoogleGemini,
            ProviderKind::OpenAiCompatible,
            ProviderKind::OpenRouter,
            ProviderKind::XAi,
            ProviderKind::OllamaNative,
            ProviderKind::FedGpt,
            ProviderKind::CustomEndpoint,
        ];

        for provider in providers {
            assert_eq!(ProviderKind::parse(provider.as_str()).unwrap(), provider);
        }
    }
}
