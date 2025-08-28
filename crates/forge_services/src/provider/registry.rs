use std::sync::Arc;
use std::{fs, io};

use anyhow::Context;
use forge_app::ProviderRegistry;
use forge_app::domain::{ModelId, Provider, ProviderUrl};
use forge_app::dto::{AppConfig, Profile};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::EnvironmentInfra;

type ProviderSearch = (&'static str, Box<dyn FnOnce(&str) -> Provider>);

#[derive(Deserialize, Clone)]
struct ProfileConfig {
    name: String,
    provider: String,
    api_key: Option<String>,
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

    fn provider_url(&self) -> Option<ProviderUrl> {
        if let Some(url) = self.infra.get_env_var("OPENAI_URL") {
            return Some(ProviderUrl::OpenAI(url));
        }

        if let Some(url) = self.infra.get_env_var("ANTHROPIC_URL") {
            return Some(ProviderUrl::Anthropic(url));
        }
        None
    }

    fn get_provider(&self, config: AppConfig) -> Option<Provider> {
        let providers = self.load_yaml().ok()?;

        // First, try to find the explicitly active provider
        let active_provider = config.profile.and_then(|active_id| {
            providers
                .iter()
                .find(|p| p.name == active_id.as_ref())
                .and_then(|def| self.config_to_provider(def))
        });

        if active_provider.is_some() {
            return active_provider;
        }

        // If no active provider, try to find the first one that can be configured
        // otherwise fallback to env provider
        providers
            .iter()
            .find_map(|def| self.config_to_provider(def))
            .or_else(|| resolve_env_provider(self.provider_url(), self.infra.as_ref()))
    }

    fn config_to_provider(&self, def: &ProfileConfig) -> Option<Provider> {
        let api_key = self.resolve_api_key(def)?;
        let mut provider = match def.provider.as_str() {
            "openai" => Provider::openai(&api_key),
            "anthropic" => Provider::anthropic(&api_key),
            "xai" => Provider::xai(&api_key),
            "openrouter" => Provider::open_router(&api_key),
            "requesty" => Provider::requesty(&api_key),
            "zai" => Provider::zai(&api_key),
            "cerebras" => Provider::cerebras(&api_key),
            _ => return None,
        };

        if let Some(base_url) = &def.base_url {
            let url = match def.provider.as_str() {
                "openai" | "xai" | "openrouter" | "requesty" | "zai" | "cerebras" => {
                    ProviderUrl::OpenAI(base_url.clone())
                }
                "anthropic" => ProviderUrl::Anthropic(base_url.clone()),
                _ => return None,
            };
            provider.url(url);
        }

        Some(provider)
    }

    fn resolve_api_key(&self, def: &ProfileConfig) -> Option<String> {
        if let Some(key) = &def.api_key {
            return Some(key.clone());
        }

        let env_var = match def.provider.as_str() {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "xai" => "XAI_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "requesty" => "REQUESTY_API_KEY",
            "zai" => "ZAI_API_KEY",
            "cerebras" => "CEREBRAS_API_KEY",
            _ => return None,
        };

        self.infra.get_env_var(env_var)
    }

    fn load_yaml(&self) -> anyhow::Result<Vec<ProfileConfig>> {
        let profile_path = self.infra.get_environment().base_path.join("profiles.yaml");
        const DEFAULT_CONFIG: &str = include_str!("../../../../profiles.default.yaml");

        let create_and_load_default = || {
            fs::write(&profile_path, DEFAULT_CONFIG).with_context(|| {
                format!(
                    "Failed to write default config to {}",
                    profile_path.display()
                )
            })?;

            serde_yml::from_str(DEFAULT_CONFIG).map_err(Into::into)
        };

        match fs::read_to_string(&profile_path) {
            Ok(content) => {
                if content.trim().is_empty() {
                    create_and_load_default()
                } else {
                    // If parsing fails, error out.
                    serde_yml::from_str(&content).with_context(|| {
                        format!("Invalid file format in {}.", profile_path.display())
                    })
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Create default
                create_and_load_default()
            }
            Err(err) => {
                Err(err).with_context(|| format!("Failed to read {}", profile_path.display()))
            }
        }
    }
}

#[async_trait::async_trait]
impl<F: EnvironmentInfra> ProviderRegistry for ForgeProviderRegistry<F> {
    async fn get_provider(&self, config: AppConfig) -> anyhow::Result<Provider> {
        if let Some(provider) = self.cache.read().await.as_ref() {
            return Ok(provider.clone());
        }

        let provider = self.get_provider(config).context("No valid provider configuration found. Please configure a profile in your `profiles.yaml` file or set one of the following environment variables: OPENROUTER_API_KEY, REQUESTY_API_KEY, XAI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY.")?;
        self.cache.write().await.replace(provider.clone());
        Ok(provider)
    }

    async fn list_profiles(&self) -> anyhow::Result<Vec<Profile>> {
        let profiles = self.load_yaml()?;

        let mut profile_list = Vec::new();

        for def in profiles {
            let provider = self.config_to_provider(&def).unwrap_or_else(|| {
                // Fallback provider if parsing fails
                Provider::openai("dummy-key")
            });

            let profile = Profile::new(def.name.clone()).provider(provider);

            let profile = if let Some(model_name) = &def.model {
                profile.model(ModelId::new(model_name))
            } else {
                profile
            };

            profile_list.push(profile);
        }

        Ok(profile_list)
    }

    async fn clear_provider_cache(&self) {
        *self.cache.write().await = None;
    }
}

fn resolve_env_provider<F: EnvironmentInfra>(
    url: Option<ProviderUrl>,
    env: &F,
) -> Option<Provider> {
    let keys: [ProviderSearch; 7] = [
        // ("FORGE_KEY", Box::new(Provider::forge)),
        ("OPENROUTER_API_KEY", Box::new(Provider::open_router)),
        ("REQUESTY_API_KEY", Box::new(Provider::requesty)),
        ("XAI_API_KEY", Box::new(Provider::xai)),
        ("OPENAI_API_KEY", Box::new(Provider::openai)),
        ("ANTHROPIC_API_KEY", Box::new(Provider::anthropic)),
        ("CEREBRAS_API_KEY", Box::new(Provider::cerebras)),
        ("ZAI_API_KEY", Box::new(Provider::zai)),
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
