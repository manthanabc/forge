use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use forge_app::domain::{Provider, ProviderUrl};
use forge_app::dto::AppConfig;
use forge_app::{ProviderInfo, ProviderRegistry};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::EnvironmentInfra;

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
        Self { infra, cache: Arc::new(Default::default()) }
    }

    fn provider_url(&self) -> Option<ProviderUrl> {
        if let Some(url) = self.infra.get_env_var("OPENAI_URL") {
            return Some(ProviderUrl::OpenAI(url));
        }
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

        let providers = self.load_yaml().ok()?;

        if let Some(active_id) = &forge_config.active_provider {
            let def = providers.get(active_id)?;
            let api_key = self.infra.get_env_var(&def.api_key_env)?;

            let mut provider = match def.provider_type.as_str() {
                "openai" => Provider::openai(&api_key),
                "anthropic" => Provider::anthropic(&api_key),
                _ => return None,
            };

            if let Some(base_url) = def.base_url.as_ref() {
                match def.provider_type.as_str() {
                    "openai" => provider.url(ProviderUrl::OpenAI(base_url.clone())),
                    "anthropic" => provider.url(ProviderUrl::Anthropic(base_url.clone())),
                    _ => {}
                }
            }
            return Some(provider);
        }

        providers.values().find_map(|def| {
            let api_key = self.infra.get_env_var(&def.api_key_env)?;
            let mut provider = match def.provider_type.as_str() {
                "openai" => Provider::openai(&api_key),
                "anthropic" => Provider::anthropic(&api_key),
                _ => return None,
            };

            if let Some(base_url) = def.base_url.as_ref() {
                match def.provider_type.as_str() {
                    "openai" => provider.url(ProviderUrl::OpenAI(base_url.clone())),
                    "anthropic" => provider.url(ProviderUrl::Anthropic(base_url.clone())),
                    _ => {}
                }
            }
            Some(provider)
        })
    }

    fn load_yaml(&self) -> anyhow::Result<HashMap<String, ProviderDefinition>> {
        let providers_path = self
            .infra
            .get_environment()
            .base_path
            .join("providers.yaml");

        if !providers_path.exists() {
            anyhow::bail!("providers.yaml not found");
        }

        let content = std::fs::read_to_string(&providers_path)?;
        let config: ProvidersYaml = serde_yml::from_str(&content)?;
        Ok(config.providers)
    }
}

#[async_trait::async_trait]
impl<F: EnvironmentInfra> ProviderRegistry for ForgeProviderRegistry<F> {
    async fn get_provider(&self, config: AppConfig) -> anyhow::Result<Provider> {
        if let Some(provider) = self.cache.read().await.as_ref() {
            return Ok(provider.clone());
        }

        let provider = self.get_provider(config).context("No valid provider configuration found. Please set one of the following environment variables: OPENROUTER_API_KEY, REQUESTY_API_KEY, XAI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY.")?;
        self.cache.write().await.replace(provider.clone());
        Ok(provider)
    }

    async fn list_providers(&self, config: AppConfig) -> anyhow::Result<Vec<ProviderInfo>> {
        let providers = match self.load_yaml() {
            Ok(providers) => providers,
            Err(_) => return Ok(Vec::new()),
        };

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

    async fn clear_cache(&self) {
        *self.cache.write().await = None;
    }
}

fn override_url(mut provider: Provider, url: Option<ProviderUrl>) -> Provider {
    if let Some(url) = url {
        provider.url(url);
    }
    provider
}
