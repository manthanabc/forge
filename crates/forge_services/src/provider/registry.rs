use std::sync::Arc;
use std::{fs, io};

use anyhow::Context;
use forge_app::ProviderRegistry;
use forge_app::domain::{ModelId, Provider, ProviderUrl, TopK};
use forge_app::dto::{AppConfig, Profile};
use forge_app::dto::profile_def::{ProfileDef, ProviderDef};
use tokio::sync::RwLock;

use crate::EnvironmentInfra;

type ProviderSearch = (&'static str, Box<dyn FnOnce(&str) -> Provider>);

pub struct ForgeProviderRegistry<F> {
    infra: Arc<F>,
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

    fn get_provider_from_profiles(&self, config: AppConfig) -> Option<Provider> {
        let profiles = self.load_yaml().ok()?;
        let mut profile_defs = profiles.iter();

        let active_profile = config.profile.and_then(|active_id| {
            profile_defs
                .clone()
                .find(|p| p.name == active_id.as_ref())
                .and_then(|def| self.convert_to_final_profile(def).ok())
        });

        if active_profile.is_some() {
            return active_profile.map(|p| p.provider);
        }

        profile_defs
            .find_map(|def| self.convert_to_final_profile(def).ok())
            .map(|p| p.provider)
            .or_else(|| resolve_env_provider(self.provider_url(), self.infra.as_ref()))
    }

    fn convert_to_final_profile(&self, def: &ProfileDef) -> anyhow::Result<Profile> {
        let api_key = self.resolve_api_key(&def.provider)?;
        let mut provider = match def.provider.provider_type.as_str() {
            "openai" => Provider::openai(&api_key),
            "anthropic" => Provider::anthropic(&api_key),
            "xai" => Provider::xai(&api_key),
            "openrouter" => Provider::open_router(&api_key),
            "requesty" => Provider::requesty(&api_key),
            "zai" => Provider::zai(&api_key),
            "cerebras" => Provider::cerebras(&api_key),
            _ => return Err(anyhow::anyhow!("Unknown provider type: {}", def.provider.provider_type)),
        };

        if let Some(base_url) = &def.provider.base_url {
            let url = match def.provider.provider_type.as_str() {
                "openai" | "xai" | "openrouter" | "requesty" | "zai" | "cerebras" => {
                    ProviderUrl::OpenAI(base_url.clone())
                }
                "anthropic" => ProviderUrl::Anthropic(base_url.clone()),
                _ => return Err(anyhow::anyhow!("Base URL not supported for provider: {}", def.provider.provider_type)),
            };
            provider.url(url);
        }

        let mut profile = Profile::new(def.name.clone()).provider(provider);
        if let Some(model) = &def.model {
            profile = profile.model(ModelId::new(model));
        }
        if let Some(top_k) = def.top_k {
            if let Ok(top_k) = TopK::new(top_k as u32) {
                profile = profile.top_k(top_k);
            }
        }

        Ok(profile)
    }

    fn resolve_api_key(&self, def: &ProviderDef) -> anyhow::Result<String> {
        if let Some(key) = &def.api_key {
            return Ok(key.clone());
        }

        let env_var = match def.provider_type.as_str() {
            "openai" => "OPENAI_API_KEY",
            "anthropic" => "ANTHROPIC_API_KEY",
            "xai" => "XAI_API_KEY",
            "openrouter" => "OPENROUTER_API_KEY",
            "requesty" => "REQUESTY_API_KEY",
            "zai" => "ZAI_API_KEY",
            "cerebras" => "CEREBRAS_API_KEY",
            _ => return Err(anyhow::anyhow!("No API key found for provider: {}", def.provider_type)),
        };

        self.infra.get_env_var(env_var).context(format!("{} not set", env_var))
    }

    fn load_yaml(&self) -> anyhow::Result<Vec<ProfileDef>> {
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
                    serde_yml::from_str(&content).with_context(|| {
                        format!("Invalid file format in {}.", profile_path.display())
                    })
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
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

        let provider = self.get_provider_from_profiles(config).context("No valid provider configuration found. Please configure a profile in your `profiles.yaml` file or set one of the following environment variables: OPENROUTER_API_KEY, REQUESTY_API_KEY, XAI_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY.")?;
        self.cache.write().await.replace(provider.clone());
        Ok(provider)
    }

    async fn list_profiles(&self) -> anyhow::Result<Vec<Profile>> {
        let profile_defs = self.load_yaml()?;
        let mut profiles = Vec::new();
        for def in profile_defs {
            // Here we ignore profiles that fail to convert, which is a reasonable strategy.
            if let Ok(profile) = self.convert_to_final_profile(&def) {
                profiles.push(profile);
            }
        }
        Ok(profiles)
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
