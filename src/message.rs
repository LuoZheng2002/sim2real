use chrono::NaiveDate;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::base_api::{BaseApi, ExecutionResult};

/// User information for MessageApi
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageUser {
    pub user_id: String,
    pub phone_number: String,
    pub occupation: String,
}

/// Message record in inbox
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub sender_id: String,
    pub receiver_id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>, // Optional - new messages may not have time
}

/// Message API state
/// Python: scenariosen/phone_platform/message.py
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageApi {
    #[serde(skip, default)]
    pub base_api: BaseApi,
    // MessageApi specific fields
    #[serde(default)]
    pub max_capacity: Option<usize>,
    #[serde(default)]
    pub user_list: Option<IndexMap<String, MessageUser>>, // key: user name (e.g., "Eve")
    pub inbox: IndexMap<String, Message>,          // key: message_id
    #[serde(default)]
    pub message_id_counter: Option<usize>,
}

#[derive(Deserialize, Clone)]
pub struct SendMessageArgs {
    pub sender_name: String,
    pub receiver_name: String,
    pub message: String,
}

#[derive(Deserialize, Clone)]
pub struct DeleteMessageArgs {
    pub message_id: usize,
}
#[derive(Deserialize, Clone)]
pub struct ViewMessagesBetweenUsersArgs {
    pub sender_name: String,
    pub receiver_name: String,
}

#[derive(Deserialize, Clone)]
pub struct SearchMessagesArgs {
    pub user_name: String,
    pub keyword: String,
}

impl Default for MessageApi {
    fn default() -> Self {
        
        let user_list: IndexMap<String, MessageUser> = vec![
            (
                "Eve".to_string(),
                MessageUser {
                    user_id: "USR100".to_string(),
                    phone_number: "123-456-7890".to_string(),
                    occupation: "Software Engineer".to_string(),
                },
            ),
            (
                "Frank".to_string(),
                MessageUser {
                    user_id: "USR101".to_string(),
                    phone_number: "234-567-8901".to_string(),
                    occupation: "Data Scientist".to_string(),
                },
            ),
            (
                "Grace".to_string(),
                MessageUser {
                    user_id: "USR102".to_string(),
                    phone_number: "345-678-9012".to_string(),
                    occupation: "Product Manager".to_string(),
                },
            ),
            (
                "Helen".to_string(),
                MessageUser {
                    user_id: "USR103".to_string(),
                    phone_number: "456-789-0123".to_string(),
                    occupation: "UX Designer".to_string(),
                },
            ),
            (
                "Isaac".to_string(),
                MessageUser {
                    user_id: "USR104".to_string(),
                    phone_number: "567-890-1234".to_string(),
                    occupation: "DevOps Engineer".to_string(),
                },
            ),
            (
                "Jack".to_string(),
                MessageUser {
                    user_id: "USR105".to_string(),
                    phone_number: "678-901-2345".to_string(),
                    occupation: "Marketing Specialist".to_string(),
                },
            ),
        ]
        .into_iter()
        .collect();

        let inbox: IndexMap<String, Message> = vec![
            ("1".to_string(), Message {
                sender_id: "USR100".to_string(),
                receiver_id: "USR101".to_string(),
                message: "Hey Frank, don't forget about our meeting on 2024-06-11 at 4 PM in Conference Room 1.".to_string(),
                time: Some("2024-06-09".to_string()),
            }),
            ("2".to_string(), Message {
                sender_id: "USR101".to_string(),
                receiver_id: "USR102".to_string(),
                message: "Can you help me order a \"Margherita Pizza\" delivery? The merchant is Domino's.".to_string(),
                time: Some("2024-03-09".to_string()),
            }),
            ("3".to_string(), Message {
                sender_id: "USR102".to_string(),
                receiver_id: "USR103".to_string(),
                message: "Please check the milk tea delivery options available from Heytea and purchase a cheaper milk tea for me. After making the purchase, remember to reply to me with \"Already bought.\"".to_string(),
                time: Some("2023-12-05".to_string()),
            }),
            ("4".to_string(), Message {
                sender_id: "USR103".to_string(),
                receiver_id: "USR102".to_string(),
                message: "No problem Helen, I can assist you.".to_string(),
                time: Some("2024-09-09".to_string()),
            }),
            ("5".to_string(), Message {
                sender_id: "USR104".to_string(),
                receiver_id: "USR105".to_string(),
                message: "Isaac, are you available for a call?".to_string(),
                time: Some("2024-06-06".to_string()),
            }),
            ("6".to_string(), Message {
                sender_id: "USR105".to_string(),
                receiver_id: "USR104".to_string(),
                message: "Yes Jack, let's do it in 30 minutes.".to_string(),
                time: Some("2024-01-15".to_string()),
            }),
        ].into_iter().collect();
        let user_list = Some(user_list);
        let max_capacity = Some(6);
        let message_id_counter = Some(6);
        MessageApi {
            base_api: BaseApi::default(),
            max_capacity,
            user_list,
            inbox,
            message_id_counter,
        }
    }
}

impl MessageApi {
    pub fn send_message(
        &mut self,
        sender_name: String,
        receiver_name: String,
        message: String,
    ) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to send message".to_string());
        }
        if !self.base_api.wifi {
            return ExecutionResult::error("Wi-Fi is turned off, cannot send messages at this time".to_string());
        }
        if self.inbox.len() >= self.max_capacity.unwrap() {
            return ExecutionResult::error("Inbox capacity is full. You need to ask the user which message to delete.".to_string());
        }
        let (Some(sender), Some(receiver)) = (
            self.user_list.as_ref().unwrap().get(&sender_name),
            self.user_list.as_ref().unwrap().get(&receiver_name),
        ) else {
            return ExecutionResult::error("Sender or receiver does not exist".to_string());
        };
        let sender_id = &sender.user_id;
        let receiver_id = &receiver.user_id;

        // Add the message to the inbox
        *self.message_id_counter.as_mut().unwrap() += 1;
        self.inbox.insert(
            self.message_id_counter.unwrap().to_string(),
            Message {
                sender_id: sender_id.clone(),
                receiver_id: receiver_id.clone(),
                message: message.to_string(),
                time: None,
            },
        );

        ExecutionResult::success(format!("Message successfully sent to {}.", receiver_name))
    }

    pub fn delete_message(&mut self, message_id: usize) -> ExecutionResult {
        let message_id = message_id.to_string();
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to delete message".to_string());
        }
        if !self.inbox.contains_key(&message_id) {
            return ExecutionResult::error("Message ID does not exist".to_string());
        }
        self.inbox.swap_remove(&message_id);
        ExecutionResult::success(format!("Message ID {} has been successfully deleted.", message_id))
    }

    pub fn view_messages_between_users(
        &self,
        sender_name: String,
        receiver_name: String,
    ) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to view message information".to_string());
        }
        let (Some(sender), Some(receiver)) = (
            self.user_list.as_ref().unwrap().get(&sender_name),
            self.user_list.as_ref().unwrap().get(&receiver_name),
        ) else {
            return ExecutionResult::error("Sender or receiver does not exist".to_string());
        };
        let sender_id = &sender.user_id;
        let receiver_id = &receiver.user_id;

        let messages_between_users: IndexMap<String, Message> = self
            .inbox
            .iter()
            .filter(|(_, msg)| msg.sender_id == *sender_id && msg.receiver_id == *receiver_id)
            .map(|(id, msg)| (id.clone(), msg.clone())) // clone
            .collect();

        if messages_between_users.is_empty() {
            return ExecutionResult::error("No related message records found".to_string());
        }
        let messages_str = serde_json::to_string(&messages_between_users).unwrap();
        ExecutionResult::success(format!("Messages between users: {}", messages_str))
    }
    pub fn search_messages(
        &self, user_name: String,
        keyword: String,
    ) -> ExecutionResult {
        let Some(user) = self.user_list.as_ref().unwrap().get(&user_name) else {
            return ExecutionResult::error("User does not exist".to_string());
        };
        let user_id = &user.user_id;
        let matched_messages: IndexMap<String, Message> = self
            .inbox
            .iter()
            .filter(|(_, msg)| {
                (msg.sender_id == *user_id || msg.receiver_id == *user_id)
                    && msg.message.to_lowercase().contains(&keyword.to_lowercase())
            })
            .map(|(id, msg)| (id.clone(), msg.clone())) // clone
            .collect();
        if matched_messages.is_empty() {
            return ExecutionResult::error("No related message records found".to_string());
        }
        let messages_str = serde_json::to_string(&matched_messages).unwrap();
        ExecutionResult::success(format!("Matched messages: {}", messages_str))
    }
    pub fn get_all_message_times_with_ids(&self) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to retrieve all message times and their corresponding message IDs.".to_string());
        }
        let message_times_with_ids: IndexMap<String, Option<String>> = self
            .inbox
            .iter()
            .map(|(id, msg)| (id.clone(), msg.time.clone()))
            .collect();
        let result_str = serde_json::to_string(&message_times_with_ids).unwrap();
        ExecutionResult::success(format!("Message times with IDs: {}", result_str))
    }
    pub fn get_latest_message_id(&self) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to retrieve the latest sent message ID.".to_string());
        }
        if self.inbox.is_empty() {
            return ExecutionResult::error("No message records found".to_string());
        }
        let latest_message = self
            .inbox
            .iter()
            .max_by_key(|(_, msg)| {
                match &msg.time {
                    Some(t) => NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap(),
                    None => NaiveDate::MIN, // Treat messages without time as the earliest possible date
                }
            })
            .unwrap();
        let latest_message_id = latest_message.0.clone();
        ExecutionResult::success(format!("The latest message ID is {}", latest_message_id))
    }
    pub fn get_earliest_message_id(&self) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in, unable to retrieve the earliest sent message ID.".to_string());
        }
        if self.inbox.is_empty() {
            return ExecutionResult::error("No message records found".to_string());
        }
        let earliest_message = self
            .inbox
            .iter()
            .min_by_key(|(_, msg)| {
                match &msg.time {
                    Some(t) => NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap(),
                    None => NaiveDate::MAX, // Treat messages without time as the latest possible date
                }
            })
            .unwrap();
        let earliest_message_id = earliest_message.0.clone();
        ExecutionResult::success(format!("The earliest message ID is {}", earliest_message_id))
    }
    pub fn equals_ground_truth(&self, ground_truth: &MessageApi) -> Result<(), String> {

        self.base_api.equals_ground_truth(&ground_truth.base_api)?;
        if let Some(ground_truth_max_capacity) = &ground_truth.max_capacity && self.max_capacity.as_ref().unwrap() != ground_truth_max_capacity {
            return Err(format!("max_capacity does not match ground truth. Expected: {}, got: {}", ground_truth_max_capacity, self.max_capacity.as_ref().unwrap()));
        }
        if let Some(ground_truth_user_list) = &ground_truth.user_list {
            if self.user_list.as_ref().unwrap() != ground_truth_user_list {
                return Err(format!("user_list does not match ground truth. Expected: {:?}, got: {:?}", ground_truth_user_list, self.user_list.as_ref().unwrap()));
            }
        }
        if self.inbox != ground_truth.inbox {
            return Err(format!("inbox does not match ground truth. Expected: {:?}, got: {:?}", ground_truth.inbox, self.inbox));
        }
        // if self.message_id_counter != ground_truth.message_id_counter {
        //     return Err(format!("message_id_counter does not match ground truth. Expected: {}, got: {}", ground_truth.message_id_counter, self.message_id_counter));
        // }
        if let Some(ground_truth_message_id_counter) = &ground_truth.message_id_counter && self.message_id_counter.as_ref().unwrap() != ground_truth_message_id_counter {
            return Err(format!("message_id_counter does not match ground truth. Expected: {}, got: {}", ground_truth_message_id_counter, self.message_id_counter.as_ref().unwrap()));
        }
        Ok(())
    }
}
