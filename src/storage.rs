extern crate task_scheduler;
#[allow(unused_parens)]
use super::announce;
use log::*;
use serenity::prelude::Context;
use std::fs;
#[feature(async_closure)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Error, Read, Write};
use std::time::Duration;
use task_scheduler::Scheduler;

pub fn save_reminder(
    timestamp: i64,
    time_to_wait: i32,
    user_id: u64,
    remind_msg: String,
) -> Result<(), Error> {
    let save_entry = format!(
        "{} {} {} {}",
        timestamp.to_string(),
        time_to_wait.to_string(),
        user_id.to_string(),
        remind_msg
    );

    let save_entry = save_entry.replace("\n", "/n");
    let save_entry = format!("{}\n", save_entry);

    info!("Save entry --> {}", save_entry);

    let path = "cache/data.txt";

    fs::create_dir_all("cache").expect("Error creating cache folder");
    if !fs::metadata(path).is_ok() {
        File::create(path).expect("Storage create failed.");
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(save_entry.as_bytes())
        .expect("Storage write failed.");

    Ok(())
}

pub async fn load_reminders(ctx_src: Context) -> Result<(), Error> {
    info!("Try load reminders list.");
    use chrono::prelude::*;
    let path = "cache/data.txt";
    use std::sync::{Arc, Mutex};

    let ctx = Arc::new(Mutex::new(ctx_src));

    if fs::metadata(path).is_ok() {
        let mut file = File::open(path).expect("File open failed");
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        let scheduler = Scheduler::new();

        let split_args = contents.split("\n").map(|x| x.to_string());

        File::create(path).expect("Storage create failed.");

        for rem in split_args {
            let cloned_ctx = Arc::clone(&ctx);

            if rem.len() > 8 {
                // println!("Loaded reminder {}", &rem.as_str());
                let mut splitter = rem.splitn(4, " ").map(|x| x.to_string());

                let timestamp = splitter
                    .next()
                    .unwrap_or_default()
                    .parse::<i64>()
                    .unwrap_or_default();
                let time_to_wait_in_seconds = splitter
                    .next()
                    .unwrap_or_default()
                    .parse::<i32>()
                    .unwrap_or_default() as i64;
                let user_id = splitter
                    .next()
                    .unwrap_or_default()
                    .parse::<u64>()
                    .unwrap_or_default();
                let remind_msg = splitter.next().unwrap_or("".to_string());

                // From https://stackoverflow.com/a/50072164/13169611
                let naive = NaiveDateTime::from_timestamp(timestamp, 0);
                let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

                let time_since_message = Utc::now().signed_duration_since(datetime).num_seconds();

                if time_since_message < time_to_wait_in_seconds {
                    println!(
                        "Schedule loaded reminder. user: {} msg: {}",
                        user_id, remind_msg
                    );

                    let final_time_wait = (time_to_wait_in_seconds - time_since_message) as u64;
                    if final_time_wait > 0 {
                        match save_reminder(
                            timestamp,
                            time_to_wait_in_seconds as i32,
                            user_id,
                            remind_msg.to_string(),
                        ) {
                            Ok(_x) => {}
                            Err(why) => {
                                error!("Error saving reminder {:?}", why);
                            }
                        };
                        scheduler.after_duration(Duration::from_secs(final_time_wait), move || {
                            // FIXME This needs to await
                            executed_reminder(user_id, remind_msg);
                        });
                    }
                }
            }
        }
    } else {
        fs::create_dir_all("cache").expect("Error creating cache folder");

        File::create(path).expect("Storage create failed.");
    }

    info!("Reminders loaded from file into memory.");

    match announce::schedule_announcements().await {
        Ok(_x) => info!("Scheduled announcements OK."),
        Err(why) => error!("Error in schedule_announcements. {:?}", why),
    };

    Ok(())
}

async fn executed_reminder(user_id: u64, remind_msg: String) {
    let ctx_http = super::globalstate::make_http();

    info!("Remind user {} about {}", user_id, remind_msg);

    let remind_msg = remind_msg.replace("/n", "\n");

    let res_user = ctx_http.get_user(user_id).await;

    match res_user {
        Ok(user_unwrapped) => {
            let dm_result = user_unwrapped
                .direct_message(&ctx_http, move |m| m.content(remind_msg))
                .await;
            match dm_result {
                Ok(_) => {}
                Err(why) => error!("Failed to send DM for stored reminder. {:?}", why),
            }
        }
        Err(why) => {
            error!(
                "Failed to retrieve user from id for stored reminder. {:?}",
                why
            );
        }
    }
}
