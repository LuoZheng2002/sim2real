use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::base_api::{BaseApi, ExecutionResult};

/// Reminder record
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reminder {
    pub reminder_id: u32,
    pub title: String,
    pub description: String,
    pub time: String, // format: "YYYY-MM-DD HH:MM"
    pub notified: bool,
}

/// Reminder API state
/// Python: scenariosen/phone_platform/reminder.py
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReminderApi {
    pub base_api: BaseApi,
    // ReminderApi specific fields
    pub max_capacity: usize,
    pub reminder_list: IndexMap<usize, Reminder>, // key: internal id (1, 2, 3...)
    pub reminder_id_counter: usize,
}

#[derive(Deserialize, Clone)]
pub struct ViewReminderByTitleArgs {
    pub title: String,
}

#[derive(Deserialize, Clone)]
pub struct AddReminderArgs {
    pub title: String,
    pub description: String,
    pub time: String,
}
#[derive(Deserialize, Clone)]
pub struct DeleteReminderArgs {
    pub reminder_id: usize,
}
impl Default for ReminderApi {
    fn default() -> Self {
        let reminder_list: IndexMap<usize, Reminder> = vec![
            (1, Reminder {
                reminder_id: 1001,
                title: "Doctor's Appointment".to_string(),
                description: "Visit Dr. Smith for a checkup.".to_string(),
                time: "2024-07-15 09:30".to_string(),
                notified: false,
            }),
            (2, Reminder {
                reminder_id: 1002,
                title: "Team Meeting".to_string(),
                description: "Monthly project review with the team.".to_string(),
                time: "2024-07-17 11:00".to_string(),
                notified: false,
            }),
            (3, Reminder {
                reminder_id: 1003,
                title: "To-do list".to_string(),
                description: "First, help Frank place a food delivery order at \"Hema Fresh,\" ordering two \"Fresh Gift Packs.\" Then, send a message to Frank saying, \"The price of the purchased goods is () yuan.\" Replace the parentheses with the actual amount, keeping one decimal place.".to_string(),
                time: "2024-07-16 11:00".to_string(),
                notified: false,
            }),
        ].into_iter().collect();
        ReminderApi {
            base_api: BaseApi::default(),
            max_capacity: 6,
            reminder_list,
            reminder_id_counter: 3,
        }
    }
}

impl ReminderApi {
    pub fn view_reminder_by_title(&self, title: String) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("The device is not logged in, so you cannot view notifications".to_string());
        }
        match self.reminder_list.values().find(|reminder| reminder.title == title) {
            Some(reminder) => ExecutionResult::success(serde_json::to_string(reminder).unwrap()),
            None => ExecutionResult::error(format!("No reminder found with the title '{}'.", title)),
        }
    }
    pub fn add_reminder(&mut self, title: String, description: String, time: String) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in. Unable to add a new reminder.".to_string());
        }
        if self.reminder_list.len() >= self.max_capacity {
            return ExecutionResult::error("Reminder capacity is full. Unable to add a new reminder.".to_string());
        }
        self.reminder_id_counter += 1;
        let reminder_id = self.reminder_id_counter;
        self.reminder_list.insert(
            reminder_id,
            Reminder {
                reminder_id: reminder_id as u32,
                title: title.to_string(),
                description: description.to_string(),
                time: time.to_string(),
                notified: false,
            },
        );
        ExecutionResult::success(format!("Reminder '{}' was successfully added.", title))
    }
    pub fn delete_reminder(&mut self, reminder_id: usize) -> ExecutionResult {
        if !self.base_api.logged_in {
            return ExecutionResult::error("Device not logged in. Unable to delete the specified reminder.".to_string());
        }
        if !self.reminder_list.contains_key(&reminder_id) {
            return ExecutionResult::error("Reminder ID does not exist.".to_string());
        }

        self.reminder_list.swap_remove(&reminder_id);
        ExecutionResult::success(format!("Reminder ID {} was successfully deleted.", reminder_id))
    }
    pub fn view_all_reminders(&self) -> ExecutionResult {
        if self.reminder_list.is_empty() {
            return ExecutionResult::error("No reminders found.".to_string());
        }
        let reminders: Vec<&Reminder> = self.reminder_list.values().collect();
        let reminders_str = serde_json::to_string(&reminders).unwrap();
        ExecutionResult::success(reminders_str)
    }
}
