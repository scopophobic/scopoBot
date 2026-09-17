/// Short, varied lines that work for every selectable companion.
pub fn line(mood: &str, index: usize) -> &'static str {
    let lines: &[&str] = match mood {
        "hover" => &[
            "Oh, hello you!",
            "Need a tiny cheerleader?",
            "I saved this spot for you.",
            "A visitor! My favorite.",
            "Yes? I'm all ears.",
            "Your tiny sidekick, reporting in!",
        ],
        "click" | "wave" => &[
            "Hi! What are we making today?",
            "One tiny high five!",
            "Happy to see you again.",
            "You've got this. I've got the waving.",
            "A little hello, just for you.",
            "I'm rooting for you!",
        ],
        "charging" | "charging_start" => &[
            "Mmm, a little power snack.",
            "Recharging my tiny ambitions.",
            "More energy, more mischief!",
            "Plugged in and feeling cozy.",
        ],
        "battery_low" | "tired" => &[
            "Could we grab a little charge?",
            "My tiny batteries need a snack.",
            "Taking a gentle breather.",
            "A recharge would be lovely.",
        ],
        "wifi_search" | "wifi_lost" => &[
            "Where did that signal wander off?",
            "Searching with my tiny antenna.",
            "Connection playing hide-and-seek?",
            "I'll keep an eye out for it.",
        ],
        "wifi_restored" => &[
            "There it is! We're back.",
            "Signal found. Tiny victory!",
            "Hello again, world!",
        ],
        "memory_high" | "memory_critical" => &[
            "RAM use is high. Close unused tabs?",
            "A little room for more ideas?",
            "Unused apps might free some RAM.",
        ],
        "cpu_high" | "cpu_critical" | "desk" => &[
            "Big task? I'll help by cheering.",
            "Your very small coworker.",
            "One little step at a time.",
            "Typing with my tiny sleeves!",
        ],
        "dance" | "headphones_idle" | "headphones_reaction" => &[
            "Ooh, a little music!",
            "This deserves a tiny dance.",
            "Good tunes, good company.",
            "Quietly vibing beside you.",
        ],
        "sleep" | "nap" => &[
            "Just five tiny minutes...",
            "Dreaming of little adventures.",
            "A cozy nap, then more fun.",
            "Saving a little energy.",
        ],
        "celebrate" | "play" => &[
            "Tiny victory dance!",
            "That deserves a happy hop.",
            "A little joy, just because.",
            "Confetti for the small wins!",
        ],
        "walk" => &[
            "Stretching my very tiny legs.",
            "A little wiggle helps.",
            "Tiny steps, big plans.",
            "Time for a sleeve stretch.",
        ],
        "curious" => &[
            "What's cooking over there?",
            "Is that our next adventure?",
            "Thinking my tiny thoughts.",
            "What should we make next?",
        ],
        _ => &[
            "Happy to keep you company.",
            "Little steps still count.",
            "Here for the cozy work hours.",
            "We make a pretty good team.",
            "A tiny friend for your big ideas.",
            "Remember to stretch your shoulders.",
            "I'm enjoying our little corner.",
            "No rush. We can take our time.",
        ],
    };
    lines[index % lines.len()]
}
