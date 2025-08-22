use std::fs;
use std::sync::Arc;

use anyhow::Context;
use forge_app::domain::{Provider, ProviderUrl};
use forge_app::dto::AppConfig;
use forge_app::{Profile, ProviderRegistry};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::EnvironmentInfra;

#[derive(Deserialize, Clone)]
struct ProfileConfig {
    name: String,
    provider: String,
    api_key: String,
    model: Option<String>,
    base_url: Option<String>,
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

    fn get_provider(&self, forge_config: AppConfig) -> Option<Provider> {
        let providers = self.load_yaml().ok()?;

        if let Some(active_id) = &forge_config.active_provider {
            let def = providers.iter().find(|p| p.name == *active_id)?;
            let api_key = def.api_key.clone();
            let mut provider = match def.provider.as_str() {
                "openai" => Provider::openai(&api_key),
                "anthropic" => Provider::anthropic(&api_key),
                "xai" => Provider::xai(&api_key),
                "openrouter" => Provider::open_router(&api_key),
                "requesty" => Provider::requesty(&api_key),
                _ => return None,
            };

            if let Some(base_url) = &def.base_url {
                let url = match def.provider.as_str() {
                    "openai" | "xai" | "openrouter" | "requesty" => {
                        ProviderUrl::OpenAI(base_url.clone())
                    }
                    "anthropic" => ProviderUrl::Anthropic(base_url.clone()),
                    _ => return None,
                };
                provider.url(url);
            }

            return Some(provider);
        }
        providers.iter().find_map(|def| {
            let api_key = def.api_key.clone();
            let mut provider = match def.provider.as_str() {
                "openai" => Provider::openai(&api_key),
                "anthropic" => Provider::anthropic(&api_key),
                "xai" => Provider::xai(&api_key),
                "openrouter" => Provider::open_router(&api_key),
                "requesty" => Provider::requesty(&api_key),
                _ => return None,
            };

            if let Some(base_url) = &def.base_url {
                let url = match def.provider.as_str() {
                    "openai" | "xai" | "openrouter" | "requesty" => {
                        ProviderUrl::OpenAI(base_url.clone())
                    }
                    "anthropic" => ProviderUrl::Anthropic(base_url.clone()),
                    _ => return None,
                };
                provider.url(url);
            }

            Some(provider)
        })
    }

    fn load_yaml(&self) -> anyhow::Result<Vec<ProfileConfig>> {
        let providers_path = self.infra.get_environment().base_path.join("profiles.yaml");

        if !providers_path.exists() {
            const DEFAULT_CONFIG: &str = include_str!("../../../../profiles.default.yaml");

            println!(
                "Configuration file not found. Created a default at: {}",
                providers_path.display()
            );

            fs::write(&providers_path, DEFAULT_CONFIG).with_context(|| {
                format!(
                    "Failed to write default config to {}",
                    providers_path.display()
                )
            })?;
        }

        let content = std::fs::read_to_string(&providers_path)?;
        let profiles: Vec<ProfileConfig> = serde_yml::from_str(&content)?;
        Ok(profiles)
    }
}

#[async_trait::async_trait]
impl<F: EnvironmentInfra> ProviderRegistry for ForgeProviderRegistry<F> {
    async fn get_provider(&self, config: AppConfig) -> anyhow::Result<Provider> {
        if let Some(provider) = self.cache.read().await.as_ref() {
            return Ok(provider.clone());
        }

        let provider = self.get_provider(config).context("No valid provider configuration found. Please configure a profile in your `profiles.yaml` file.")?;
        self.cache.write().await.replace(provider.clone());
        Ok(provider)
    }

    async fn list_profiles(&self, config: AppConfig) -> anyhow::Result<Vec<Profile>> {
        let profiles = match self.load_yaml() {
            Ok(profiles) => profiles,
            Err(_) => return Ok(Vec::new()),
        };

        let active_provider_id = config.active_provider.as_ref();
        let mut profile_list = Vec::new();

        for def in profiles {
            let is_active = active_provider_id == Some(&def.name);

            profile_list.push(forge_app::Profile {
                name: def.name.clone(),
                provider: def.provider,
                is_active,
                model_name: def.model,
            });
        }

        Ok(profile_list)
    }

    async fn clear_provider_cache(&self) {
        *self.cache.write().await = None;
    }
}
