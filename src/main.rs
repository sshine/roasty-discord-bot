use rand::{rngs::SmallRng, Rng as _, SeedableRng};
use std::{env, error::Error, sync::Arc};
use tokio::sync::Mutex;
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, Shard, ShardId};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::Intents;

struct MainState {
    rng: SmallRng,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let token = env::var("DISCORD_API_KEY")?;

    // Use intents to only receive guild message events.
    let mut shard = Shard::new(
        ShardId::ONE,
        token.clone(),
        Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT,
    );

    // HTTP is separate from the gateway, so create a new client.
    let http = Arc::new(HttpClient::new(token));

    // Since we only care about new messages, make the cache only
    // cache new messages.
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    let main_state = Arc::new(Mutex::new(MainState {
        rng: SmallRng::from_entropy(),
    }));

    // Process each event as they come in.
    loop {
        let event = match shard.next_event().await {
            Ok(event) => event,
            Err(source) => {
                tracing::warn!(?source, "error receiving event");

                if source.is_fatal() {
                    break;
                }

                continue;
            }
        };

        // Update the cache with the event.
        cache.update(&event);

        tokio::spawn(handle_event(event, main_state.clone(), Arc::clone(&http)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    main_state_lock: Arc<Mutex<MainState>>,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) => {
            if msg.content.contains("d6") {
                let mut main_state = main_state_lock.lock().await;
                let roll = main_state.rng.gen_range(1..=6);
                tracing::info!(roll, "roll");
                let response = format!("{}", roll);
                http.create_message(msg.channel_id)
                    .content(&response)?
                    .await?;
            }
        }

        Event::MessageUpdate(_msg) => {
            // do nothing
        }

        _ => {
            // do nothing
        }
    }

    Ok(())
}
