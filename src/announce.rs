#[allow(unused_parens)]
use super::globalstate;

use chrono::Utc;
use serenity::model::id::ChannelId;
use std::io::Error;
extern crate job_scheduler;
use job_scheduler::{Job, JobScheduler};
use log::*;
#[feature(async_closure)]
use serenity::prelude::Context;
use std::time::Duration;

pub async fn schedule_announcements() {
    let mut sched_worklog = JobScheduler::new();
    let mut sched_qa_dev_reminder = JobScheduler::new();
    let mut sched_qa_day = JobScheduler::new();

    sched_worklog.add(Job::new(
        "1/15 * * * * *".parse().unwrap(),
        Box::new(|| {
            Box::pin(async {
                check_work_log().await;
            })
        }),
    ));

    sched_qa_dev_reminder.add(Job::new(
        "1/10 * * * * *".parse().unwrap(),
        Box::new(|| {
            Box::pin(async {
                send_qa_day_dev_reminder().await;
            })
        }),
    ));

    sched_qa_day.add(Job::new(
        "1/20 * * * * *".parse().unwrap(),
        Box::new(|| {
            Box::pin(async {
                send_qa_day_all_reminder().await;
            })
        }),
    ));

    info!("Scheduled announcement, now entering scheduler loop ...");

    loop {
        sched_worklog.tick().await;
        sched_qa_dev_reminder.tick().await;
        sched_qa_day.tick().await;

        std::thread::sleep(Duration::from_millis(500));
    }

}

pub async fn send_qa_day_dev_reminder() -> Result<(), Error> {
    let ctx_http = globalstate::make_http();

    let dev_reminder_channel_id: u64 = 705037778471223339; // Real

    let dev_reminder_channel_id: u64 = 775708031668977666; // Test

    let dev_reminder_chan = ctx_http.get_channel(dev_reminder_channel_id).await;
    let dev_role_id: u64 = 705034249652273153;

    let msg_dev_remider = format!(
        "Tomorrow is QA Day! Get your test builds ready <@&{}> !",
        dev_role_id.to_string()
    );

    match dev_reminder_chan {
        Ok(x) => {
            match x.guild() {
                Some(guild_lock) => {
                    let say_res = guild_lock.say(&ctx_http, msg_dev_remider).await;
                    match say_res {
                        Ok(_x) => {}
                        Err(why) => {
                            error!("Error saying message to dev reminder channel. {:?}", why);
                        }
                    }
                }
                None => {
                    warn!("It's not a guild!");
                }
            };
        }
        Err(why) => {
            error!("Error getting dev reminder channel. {:?}", why);
        }
    };

    Ok(())
}

pub async fn send_qa_day_all_reminder() -> Result<(), Error> {
    let ctx_http = globalstate::make_http();

    let dev_reminder_channel_id: u64 = 705090277794119790; // Real

    let dev_reminder_channel_id: u64 = 775708031668977666; // Test

    let dev_reminder_chan = ctx_http.get_channel(dev_reminder_channel_id).await;

    let msg_dev_remider = format!("Today is QA Day! Happy testing @here !",);

    match dev_reminder_chan {
        Ok(x) => {
            match x.guild() {
                Some(guild_lock) => {
                    let say_res = guild_lock.say(&ctx_http, msg_dev_remider).await;
                    match say_res {
                        Ok(_x) => {}
                        Err(why) => {
                            error!("Error saying message to QA reminder channel. {:?}", why);
                        }
                    }
                }
                None => {
                    warn!("It's not a guild!");
                }
            };
        }
        Err(why) => {
            error!("Error sending message to QA reminder channel. {:?}", why);
        }
    };

    Ok(())
}

pub async fn check_work_log() -> Result<(), Error> {
    let ctx_http = globalstate::make_http();

    let worklog_channel_id: u64 = 705067423530745957; // Real

    let worklog_channel_id: u64 = 775708031668977666; // Test

    let channel_id = ChannelId(worklog_channel_id);

    let all_devs: Vec<u64> = vec![
        492385983833047051,
        503494040436604930,
        447503701733539845,
        305360713893937162,
        669148598193225739,
    ]; // sam 669148598193225739
    let mut did_speak = vec![];
    let mut didnt_speak = vec![];
    let _messages = channel_id.messages(&ctx_http, |retriever| retriever).await;
    let mut didnt_size: u64 = 0;
    let mut did_size: u64 = 0;
    match _messages {
        Ok(msgs) => {
            for dev in all_devs {
                let mut dev_spoke = false;
                let mut words_count = 0;
                for elem in &msgs {
                    if (
                        (&dev == elem.author.id.as_u64())
                            && ((Utc::now().timestamp() - elem.timestamp.timestamp()) < 432000)
                        // week 604800
                    ) {
                        let mut words = elem.content.split_whitespace();
                        let local_words_count = words.count();
                        if local_words_count > 2 {
                            words_count += local_words_count;
                        }
                    };
                }
                if words_count > 5 {
                    dev_spoke = true;
                }
                if dev_spoke {
                    did_speak.push(dev);
                } else {
                    didnt_speak.push(dev);
                }
            }
            didnt_size = didnt_speak.len() as u64;
            did_size = did_speak.len() as u64;
        }
        Err(why) => {
            error!("Failed to get messages. {:?}", why);
        }
    }

    let work_log_channel = ctx_http.get_channel(worklog_channel_id).await;

    match work_log_channel {
        Ok(x) => {
            match x.guild() {
                Some(guild_lock) => {
                    if didnt_size > 0 {
                        let mut msg_didnt_worklog =
                            "remember to post your weekly progress!".to_string();

                        for elem in didnt_speak {
                            let str_mention = format!("<@{}> ", &elem.to_string());
                            msg_didnt_worklog =
                                format!("{}{}", &str_mention, &msg_didnt_worklog).to_owned();
                        }
                        let say_res = guild_lock.say(&ctx_http, msg_didnt_worklog).await;
                        match say_res {
                            Ok(_x) => {}
                            Err(why) => {
                                error!("Error saying message to work log channel. {:?}", why);
                            }
                        }
                    }
                    if did_size > 0 {
                        let mut names_added: u32 = 0;
                        let mut msg_did_worklog = "posted in the past week, congrats!".to_string();

                        for elem in did_speak {
                            let user_res = &ctx_http.get_user(elem).await;
                            match user_res {
                                Ok(x_user) => {
                                    names_added += 1;
                                    let mut user_nick =
                                        x_user.tag().split('#').collect::<Vec<&str>>()[0]
                                            .to_string();
                                    let nick_res =
                                        x_user.nick_in(&ctx_http, 704822217237856298).await;
                                    match nick_res {
                                        Some(x_nick) => {
                                            user_nick = x_nick;
                                        }
                                        None => {}
                                    };

                                    msg_did_worklog = format!("{} {}", user_nick, &msg_did_worklog);
                                }
                                Err(why) => {
                                    error!("Error getting user with id {}. {:?}", elem, why);
                                }
                            };
                        }
                        if names_added > 0 {
                            let say_res = guild_lock.say(&ctx_http, msg_did_worklog).await;
                            match say_res {
                                Ok(_x) => {}
                                Err(why) => {
                                    error!("Error saying message to work log channel. {:?}", why);
                                }
                            }
                        }
                    }
                }
                None => {
                    warn!("It's not a guild!");
                }
            };
        }
        Err(why) => {
            error!("Error sending message to worklog channel. {:?}", why);
        }
    };

    Ok(())
}
