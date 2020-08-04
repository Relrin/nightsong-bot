// Special module that contains various
// formatters for the giveaway rewards
use std::sync::Arc;

use crate::commands::giveaway::formatters::base::RewardFormatter;
use crate::commands::giveaway::models::{ObjectState, ObjectType, Reward};

pub struct DefaultRewardFormatter;

impl DefaultRewardFormatter {
    pub fn new() -> Self {
        DefaultRewardFormatter {}
    }

    // Replaces the last part of the key into `x` symbols to stop abusing
    // exposed keys in giveaways.
    fn generate_key_with_mask(&self, reward: &Arc<Box<Reward>>) -> Arc<String> {
        let key_fragments = reward
            .value()
            .split('-')
            .map(|key_fragment| key_fragment.to_string())
            .collect::<Vec<String>>();
        let parts_count = key_fragments.len();
        let key_with_mask = key_fragments
            .into_iter()
            .enumerate()
            .map(|(index, key_fragment)| match index == parts_count - 1 {
                true => key_fragment.chars().map(|_| 'x').collect::<String>(),
                false => key_fragment,
            })
            .collect::<Vec<String>>()
            .join("-");
        Arc::new(key_with_mask)
    }
}

impl RewardFormatter for DefaultRewardFormatter {
    // Returns detailed info for the giveaway owner when necessary to update the giveaway.
    fn debug_print(&self, reward: &Arc<Box<Reward>>) -> String {
        match reward.object_type() {
            ObjectType::Key | ObjectType::KeyPreorder => {
                let key = match reward.object_info() {
                    Some(info) => format!("{} {}", reward.value(), info),
                    None => format!("{}", reward.value()),
                };

                format!(
                    "{} -> {}",
                    key,
                    reward.description().unwrap_or(String::from("")),
                )
            }
            ObjectType::Other => format!(
                "{}{}",
                reward.value(),
                reward.description().unwrap_or(String::from("")),
            ),
        }
    }

    // Stylized print for the users in the channel when the giveaways has been started.
    fn pretty_print(&self, reward: &Arc<Box<Reward>>) -> String {
        let text = match reward.object_type() {
            // Different output of the key, depends on the current state
            ObjectType::Key | ObjectType::KeyPreorder => {
                let masked_key = match reward.object_state() == ObjectState::Unused {
                    true => self.generate_key_with_mask(reward),
                    false => reward.value(),
                };

                let key = match reward.object_info() {
                    Some(info) => format!("{} {}", masked_key, info),
                    None => format!("{}", masked_key),
                };

                match reward.object_state() {
                    // When is Activated show what was hidden behind the key
                    ObjectState::Activated => format!(
                        "{} {} -> {}",
                        reward.object_state().as_str(),
                        key,
                        reward.description().unwrap_or(String::from("")),
                    ),
                    // For Unused/Pending states print minimal amount of info
                    _ => format!("{} {}", reward.object_state().as_str(), key),
                }
            }
            // Print any non-keys as is
            ObjectType::Other => format!(
                "{} {}{}",
                reward.object_state().as_str(),
                reward.value(),
                reward.description().clone().unwrap_or(String::from("")),
            ),
        };

        // If the object was taken by someone, then cross out the text
        match reward.object_state() == ObjectState::Activated {
            true => format!("~~{}~~", text),
            false => text,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::commands::giveaway::formatters::{DefaultRewardFormatter, RewardFormatter};
    use crate::commands::giveaway::models::{ObjectState, Reward};

    #[test]
    fn test_default_pretty_print_for_the_reward_in_the_unused_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Arc::new(Box::new(Reward::new(text)));
        let formatter = DefaultRewardFormatter::new();

        let output = formatter.pretty_print(&reward);
        assert_eq!(output, "[ ] AAAAA-BBBBB-CCCCC-xxxx [Store]");
    }

    #[test]
    fn test_default_pretty_print_for_the_reward_in_the_pending_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Arc::new(Box::new(Reward::new(text)));
        let formatter = DefaultRewardFormatter::new();

        reward.set_object_state(ObjectState::Pending);
        let output = formatter.pretty_print(&reward);
        assert_eq!(output, "[?] AAAAA-BBBBB-CCCCC-DDDD [Store]");
    }

    #[test]
    fn test_default_pretty_print_for_the_reward_in_the_activated_state() {
        let text = "AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game";
        let reward = Arc::new(Box::new(Reward::new(text)));
        let formatter = DefaultRewardFormatter::new();

        reward.set_object_state(ObjectState::Activated);
        let output = formatter.pretty_print(&reward);
        assert_eq!(
            output,
            "~~[+] AAAAA-BBBBB-CCCCC-DDDD [Store] -> Some game~~"
        );
    }

    #[test]
    fn test_default_pretty_print_for_an_unknown_object_in_the_unused_state() {
        let text = "just a text";
        let reward = Arc::new(Box::new(Reward::new(text)));
        let formatter = DefaultRewardFormatter::new();

        let output = formatter.pretty_print(&reward);
        assert_eq!(output, "[ ] just a text");
    }

    #[test]
    fn test_default_pretty_print_for_an_unknown_object_in_the_activated_state() {
        let text = "just a text";
        let reward = Arc::new(Box::new(Reward::new(text)));
        let formatter = DefaultRewardFormatter::new();

        reward.set_object_state(ObjectState::Activated);
        let output = formatter.pretty_print(&reward);
        assert_eq!(output, "~~[+] just a text~~");
    }
}
