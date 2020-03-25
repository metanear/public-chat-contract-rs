use borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{Vector, Map};
use near_sdk::{env, near_bindgen};
use serde::{Deserialize, Serialize};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

type AppId = String;
type Key = String;
type Value = String;
type AccountId = String;
type ChannelId = String;
type ChannelHash = Vec<u8>;

const CHAT_APP_ID: &[u8] = b"chat";

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct MetanearChat {
    channels: Map<ChannelHash, Channel>,
    total_num_messages: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Channel {
    channel_id: ChannelId,
    messages: Vector<Message>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize)]
pub struct Message {
    /// Time in nanoseconds.
    time: u64,
    /// The account Id of the message sender.
    sender_id: AccountId,
    /// The content of the message.
    text: String,
}

#[derive(Deserialize)]
pub enum GetRequest {
    Status {},
    ChannelStatus {
        channel_id: ChannelId
    },
    ChannelMessages {
        channel_id: ChannelId,
        from_index: u64,
        limit: u64,
    }
}

#[derive(Serialize)]
pub struct StatusResponse {
    num_channels: u64,
    total_num_messages: u64,
}

#[derive(Serialize)]
pub struct ChannelStatusResponse {
    num_messages: u64,
}

#[derive(Serialize)]
pub struct ChannelMessagesResponse {
    messages: Vec<Message>,
}


#[derive(Deserialize)]
pub enum IncomingMessage {
    ChatMessage {
        channel_id: ChannelId,
        text: String,
    }
}

fn verify_app_id(app_id: &AppId) {
    if app_id.len() < 2 || app_id.len() > 64 {
        env::panic(b"App ID length should be between 2 and 64 characters");
    }
    for c in app_id.bytes() {
        match c {
            b'a'..=b'z' => (),
            b'0'..=b'9' => (),
            b'-' | b'_' | b'.' => (),
            _ => env::panic(
                b"Unsupported character in the app ID. Only allowed to use `-.|` and 0-9 a-z",
            ),
        }
    }
}

fn verify_channel_id(channel_id: &ChannelId) {
    if channel_id.len() < 1 || channel_id.len() > 128 {
        env::panic(b"Channel length should be between 1 and 128 characters");
    }
    for c in channel_id.bytes() {
        match c {
            b'a'..=b'z' => (),
            b'0'..=b'9' => (),
            b'-' | b'_' | b'.' => (),
            _ => env::panic(
                b"Unsupported character in the channel. Only allowed to use `-.|` and 0-9 a-z",
            ),
        }
    }
}

fn app_key(app_id: &AppId, key: &Key) -> Vec<u8> {
    let app_id_hash = env::sha256(app_id.as_bytes());
    let key_hash = env::sha256(key.as_bytes());
    let mut res = Vec::with_capacity(app_id_hash.len() + key_hash.len() + 1);
    res.push(b'a');
    res.extend(app_id_hash);
    res.extend(key_hash);
    res
}


fn messages_key_from_hash(channel_hash: ChannelHash) -> Vec<u8> {
    let mut res = Vec::with_capacity(channel_hash.len() + 1);
    res.push(b'm');
    res.extend_from_slice(&channel_hash);
    res
}

impl Default for MetanearChat {
    fn default() -> Self {
        env::panic(b"Not initialized yet.");
    }
}

fn assert_self() {
    assert_eq!(env::current_account_id(), env::predecessor_account_id(), "Self calls only");
}

#[near_bindgen]
impl MetanearChat {
    #[init]
    pub fn new() -> Self {
        assert!(env::state_read::<MetanearChat>().is_none(), "The contract is already initialized");
        Self {
            channels: Map::new(b"c".to_vec()),
            total_num_messages: 0,
        }
    }

    pub fn master_set(&mut self, app_id: AppId, key: Key, value: Value) {
        assert_self();
        env::storage_write(&app_key(&app_id, &key), &value.as_bytes());
    }

    pub fn master_remove(&mut self, app_id: AppId, key: Key) {
        assert_self();
        env::storage_remove(&app_key(&app_id, &key));
    }

    pub fn get(&self, app_id: AppId, key: Key) -> Option<Value> {
        verify_app_id(&app_id);
        if app_id.as_bytes() == CHAT_APP_ID {
            let request: GetRequest = serde_json::from_str(&key).expect("Can't parse key request");
            match request {
                GetRequest::Status {} => {
                    Some(serde_json::to_string(&StatusResponse {
                        num_channels: self.channels.len(),
                        total_num_messages: self.total_num_messages,
                    }).unwrap())
                },
                GetRequest::ChannelStatus { channel_id } => {
                    let channel = self.get_channel(channel_id);
                    Some(serde_json::to_string(&ChannelStatusResponse {
                        num_messages: channel.messages.len(),
                    }).unwrap())
                },
                GetRequest::ChannelMessages { channel_id, from_index, limit } => {
                    let channel = self.get_channel(channel_id);
                    let mut messages = Vec::new();
                    let mut index = from_index;
                    while (messages.len() as u64) < limit && index < channel.messages.len() {
                        messages.push(channel.messages.get(index).unwrap());
                        index += 1;
                    }
                    Some(serde_json::to_string(&ChannelMessagesResponse {
                        messages,
                    }).unwrap())
                },
            }
        } else {
            env::storage_read(&app_key(&app_id, &key)).map(|bytes| String::from_utf8(bytes).unwrap())
        }
    }

    /// Called when receiving a message
    pub fn post_message(&mut self, app_id: AppId, message: String) {
        verify_app_id(&app_id);
        assert_eq!(app_id.as_bytes(), CHAT_APP_ID, "I only support chat messages");

        let sender_id = env::predecessor_account_id();

        let incoming_message: IncomingMessage = serde_json::from_str(&message).expect("Can't parse the message");
        match incoming_message {
            IncomingMessage::ChatMessage { channel_id, text } => {
                let mut channel = self.get_channel(channel_id);
                channel.add_message(sender_id, text);
                self.save_channel(&channel);
                self.total_num_messages += 1;
            },
        };
    }
}

impl MetanearChat {
    pub fn get_channel(&self, channel_id: ChannelId) -> Channel {
        verify_channel_id(&channel_id);
        let channel_hash = env::sha256(channel_id.as_bytes());
        self.channels.get(&channel_hash).unwrap_or_else(|| Channel::new(channel_id))
    }

    pub fn save_channel(&mut self, channel: &Channel) {
        let channel_hash = env::sha256(channel.channel_id.as_bytes());
        self.channels.insert(&channel_hash, &channel);
    }
}


impl Channel {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            messages: Vector::new(messages_key_from_hash(env::sha256(channel_id.as_bytes()))),
            channel_id,
        }
    }

    pub fn add_message(&mut self, sender_id: AccountId, text: String) {
        self.messages.push(&Message {
            sender_id,
            text,
            time: env::block_timestamp() / 1000000,
        });
    }
}


#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_bindgen::MockedBlockchain;
    use near_bindgen::{testing_env, VMContext};

    fn alice() -> String {
        "alice.near".to_string()
    }

    fn bob() -> String {
        "bob.near".to_string()
    }

    fn carol() -> String {
        "carol.near".to_string()
    }

    fn get_context(signer_account_pk: Vec<u8>) -> VMContext {
        VMContext {
            current_account_id: alice(),
            signer_account_id: alice(),
            signer_account_pk,
            predecessor_account_id: alice(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
        }
    }
}
