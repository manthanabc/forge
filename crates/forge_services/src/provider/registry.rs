use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use forge_app::domain::{Provider, ProviderUrl};
use forge_app::dto::AppConfig;
use forge_app::{ProviderInfo, ProviderRegistry};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::EnvironmentInfra;

type ProviderSearch = (&'static str, Box<dyn FnOnce(&str) -> Provider>);

#[derive(Deserialize, Clone)]
struct ProvidersYaml {
    providers: HashMap<String, ProviderDefinition>,
}

#[derive(Deserialize, Clone)]
struct ProviderDefinition {
    name: String,
    #[serde(rename = "type")]
    provider_type: String,
    #[serde(default)]
    base_url: Option<String>,
    api_key_env: String,
}

pub struct ForgeProviderRegistry<F> {
    infra: Arc<F>,
    // IMPORTANT: This cache is used to avoid logging out if the user has logged out from other
    // session. This helps to keep the user logged in for current session.
    cache: Arc<RwLock<Option<Provider>>>,
}

impl<F: EnvironmentInfra> ForgeProviderRegistry<F> {
    pub fn new(infra: Arc<F>) -> Self {
        Self {
            infra,
            cache: Arc::new(Default::default()),
        }
    }

    fn provider_url(&self) -> Option<ProviderUrl> {
        if let Some(url) = self.infra.get_env_var("OPENAI_URL") {
            return Some(ProviderUrl::OpenAI(url));
        }

        // Check for Anthropic URL override
        if let Some(url) = self.infra.get_env_var("ANTHROPIC_URL") {
            return Some(ProviderUrl::Anthropic(url));
        }
        None
    }
    fn get_provider(&self, forge_config: AppConfig) -> Option<Provider> {
        if let Some(forge_key) = &forge_config.key_info {
            let provider = Provider::forge(forge_key.api_key.as_str());
            return Some(override_url(provider, self.provider_url()));
        }

        // If active provider present in app config
        if let Some(active_id) = &forge_config.active_provider
            && let Ok(provider) = self.load_provider_from_yaml(active_id) {
                return Some(provider);
            }

        resolve_env_provider(self.provider_url(), self.infra.as_ref())
    }

    fn load_provider_from_yaml(&self, provider_id: &str) -> anyhow::Result<Provider> {
        let providers = self.load_providers_yaml()?;

        let provider_def = providers.get(provider_id).ok_or_else(|| {
            anyhow::anyhow!("Provider '{}' not found in providers.yaml", provider_id)
        })?;

        self.create_provider_from_def(provider_def)
    }

    fn load_providers_yaml(&self) -> anyhow::Result<HashMap<String, ProviderDefinition>> {
        let providers_path = self
            .infra
            .get_environment()
            .base_path
            .join("providers.yaml");

        if !providers_path.exists() {
            anyhow::bail!("providers.yaml not found at {}", providers_path.display());
        }

        let content = std::fs::read_to_string(&providers_path).with_context(|| {
            format!(
                "Failed to read providers.yaml from {}",
                providers_path.display()
            )
        })?;

        let config: ProvidersYaml = serde_yml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse providers.yaml from {}",
                providers_path.display()
            )
        })?;

        Ok(config.providers)
    }
    fn create_provider_from_def(&self, def: &ProviderDefinition) -> anyhow::Result<Provider> {
        let api_key = self.infra.get_env_var(&def.api_key_env).ok_or_else(|| {
            anyhow::anyhow!(
                "Environment variable '{}' not found for provider",
                def.api_key_env
            )
        })?;

        let mut provider = match def.provider_type.as_str() {
            "openai" => Provider::openai(&api_key),
            "anthropic" => Provider::anthropic(&api_key),
            _ => anyhow::bail!("Unsupported provider type: {}", def.provider_type),
        };

        // Override URL if specified
        if let Some(base_url) = &def.base_url {
            match def.provider_type.as_str() {
                "openai" => provider.url(ProviderUrl::OpenAI(base_url.clone())),
                "anthropic" => provider.url(ProviderUrl::Anthropic(base_url.clone())),
                _ => {}
            }
        }

        Ok(provider)
    }
}

#[async_trait::async_trait]
impl<F: EnvironmentInfra> ProviderRegistry for ForgeProviderRegistry<F> {
    async fn get_provider(&self, config: AppConfig) -> anyhow::Result<Provider> {
        if let Some(provider) = self.cache.read().await.as_ref() {
            return Ok(provider.clone());
        }

        let provider = self
            .get_provider(config)
            .context("Failed to detect upstream provider")?;
        self.cache.write().await.replace(provider.clone());
        Ok(provider)
    }

    async fn list_providers(&self, config: AppConfig) -> anyhow::Result<Vec<ProviderInfo>> {
        // Try to load providers from YAML
        let providers = match self.load_providers_yaml() {
            Ok(providers) => providers,
            Err(_) => {
                // No providers.yaml file exists, return empty list
                return Ok(Vec::new());
            }
        };

        // Get active provider from config
        let active_provider_id = config.active_provider.as_ref();

        let mut provider_list = Vec::new();
        for (id, def) in providers {
            let has_api_key = self.infra.get_env_var(&def.api_key_env).is_some();
            let is_active = active_provider_id == Some(&id);

            provider_list.push(ProviderInfo {
                id: id.clone(),
                name: def.name,
                provider_type: def.provider_type,
                base_url: def.base_url,
                has_api_key,
                is_active,
            });
        }

        Ok(provider_list)
    }
}

fn resolve_env_provider<F: EnvironmentInfra>(
    url: Option<ProviderUrl>,
    env: &F,
) -> Option<Provider> {
    let keys: [ProviderSearch; 6] = [
        ("FORGE_KEY", Box::new(Provider::forge)),
        ("OPENROUTER_API_KEY", Box::new(Provider::open_router)),
        ("REQUESTY_API_KEY", Box::new(Provider::requesty)),
        ("XAI_API_KEY", Box::new(Provider::xai)),
        ("OPENAI_API_KEY", Box::new(Provider::openai)),
        ("ANTHROPIC_API_KEY", Box::new(Provider::anthropic)),
    ];

    keys.into_iter().find_map(|(key, fun)| {
        env.get_env_var(key).map(|key| {
            let provider = fun(&key);
            override_url(provider, url.clone())
        })
    })
}

fn override_url(mut provider: Provider, url: Option<ProviderUrl>) -> Provider {
    if let Some(url) = url {
        provider.url(url);
    }
    provider
}
