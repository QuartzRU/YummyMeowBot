use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, sync::Arc, time::Duration as StdDuration};
use teloxide::{
    prelude::*,
    types::{
        ChatPermissions, InlineKeyboardButton, InlineKeyboardMarkup, MessageEntityKind, ParseMode,
    },
    utils::command::BotCommands,
};
use tokio::sync::Mutex;
use url::Url;

const DATA_FILE: &str = "bot_data.json";

const RULES_TEXT: &str = "\
<b>Правила группы:</b>

<b>1. Общие правила.</b>

<b>Запрещено:</b>

<b>1.1 Оскорбления.</b>
Оскорбления других участников по любым признакам.

<b>1.2 Конфликты.</b>
Создавать, провоцировать или поддерживать конфликты, проявлять неуважение к другим участникам, а также заниматься агрессивным троллингом, направленным на разжигание ненависти, вражды или создание конфликтной обстановки.

<b>1.3 Неподобающие темы.</b>
Обсуждение тем, способных привести к нежелательным последствиям или конфликтам, в том числе:
— расизм и национализм;
— религия;
— политика и идеологии;
— суицид и самоуничижение.
Темы, которые могут негативно повлиять на пользователей младше 18 лет:
— упоминание наркотиков, даже в скрытой форме;
— азартные игры и ставки;
— призыв к употреблению алкогольной и табачной продукции.

<b>1.4 Некорректная подача.</b>
Бессмыслица — контент, не несущий смысловой нагрузки.
Капс — злоупотребление верхним регистром.
Флуд — малосодержательный и нетематический контент.
Спам — массовая рассылка.
Мультипостинг — множество сообщений, которые могут быть объединены в одно.
Многократное повторение фраз, символов, эмодзи, стикеров и картинок.
Публиковать NSFW-контент.
Призывать к интимной близости/показу интимных частей тела.
Косвенно или прямо домогаться других участников.
Непристойно описывать анатомические реакции интимного характера.

<b>1.6 Спойлеры.</b>
Публиковать спойлеры без соответствующей пометки.

<b>1.7 Злоупотребление матом.</b>
Чрезмерное использование нецензурной и грубой лексики, занимающей большую часть контента.

<b>1.8 Реклама.</b>
Рекламировать любой контент без согласования на почте для сотрудничества promotion@yummyani.me
Упоминать другие сайты со схожей тематикой.
Рекомендовать контент, нарушающий правила площадки.
Агрессивно навязывать услуги.

<b>1.9 Введение в заблуждение.</b>
Выдавать себя за другого пользователя или представителя команды проекта, а также распространять ложную информацию о YummyAnime.

<b>1.10 Преследование и давление.</b>
Навязчивое преследование на сайте и других ресурсах, в том числе:
— массовые дизлайки;
— травля и высмеивание;
— разглашение или использование личной информации пользователей без их согласия.

<b>1.11 Претензии.</b>
Обвинение модераторов в их работе (оспорить решение модераторов можно только в поддержке).
Претензии к функционалу сайта или упрёки в сторону разработчиков.
Споры насчёт любых пунктов правил.

<b>1.12 Вредительство.</b>
Любые умышленные действия, направленные на причинение вреда проекту, нарушение его работы или саботаж деятельности команды.

<b>2. Аккаунты и профили.</b>

<b>2.1 Количество аккаунтов.</b>
Количество аккаунтов не ограничено. Запрещено использовать дополнительные аккаунты для накрутки, нарушения правил или обхода ограничений. Такие аккаунты могут быть заблокированы навсегда, а к основному аккаунту применены дополнительные ограничения.

<b>2.2 Оформление профиля.</b>
Имя пользователя, аватар и другие элементы профиля не должны нарушать основные правила.

<b>3. Модерация и наказания.</b>

<b>3.1 Жалобы.</b>
Каждый пользователь вправе подать жалобу на нарушение правил. Запрещено злоупотреблять системой жалоб. Необоснованные жалобы могут повлечь наказание.

<b>3.2 Возможности модерации.</b>
Модератор сам определяет вид и срок наказания в зависимости от нарушения, включая удаление контента, ограничение функций площадки и временную или постоянную блокировку пользователя.

<b>3.3 Повторные нарушения.</b>
При повторных нарушениях правил срок и строгость наказания могут быть увеличены.

<b>3.4 Апелляция.</b>
Оспорить наказание можно только через поддержку в Telegram или на сайте, описав ситуацию и предоставив доказательства. При удовлетворении апелляции наказание будет изменено или отменено.
Запрещены подделка доказательств и спам с просьбами снять наказание в поддержку или в ЛС модераторов.

<b>3.5 Помеха в работе отделов.</b>
Запрещено нарушать установленные правила публикации, добавления и редактирования материалов проекта, а также намеренно препятствовать работе модерации и других отделов проекта.";

#[derive(Serialize, Deserialize, Clone, Debug)]
struct CachedUser {
    id: u64,
    name: String,
    #[serde(default)]
    username: Option<String>,
}

impl CachedUser {
    fn display_mention(&self) -> String {
        match &self.username {
            Some(uname) if !uname.is_empty() => format!("@{}", uname.trim_start_matches('@')),
            _ => self.name.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct BotState {
    users: HashMap<String, CachedUser>,
    warns: HashMap<String, u8>,
}

impl BotState {
    fn load() -> Self {
        fs::read_to_string(DATA_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(DATA_FILE, json);
        }
    }

    fn remember_user(&mut self, user: &teloxide::types::User) -> bool {
        if let Some(username) = &user.username {
            let key = username.to_lowercase();
            let entry = CachedUser {
                id: user.id.0,
                name: user.first_name.clone(),
                username: Some(username.clone()),
            };
            let changed = self
                .users
                .get(&key)
                .map(|u| u.id != entry.id || u.name != entry.name || u.username != entry.username)
                .unwrap_or(true);
            if changed {
                self.users.insert(key, entry);
                return true;
            }
        }
        false
    }
}

type SharedState = Arc<Mutex<BotState>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Доступные команды:")]
enum Command {
    #[command(description = "показать правила группы")]
    Rules,
    #[command(description = "ограничить отправку сообщений: /mute @user 10m или ответом")]
    Mute(String),
    #[command(description = "снять ограничения: /unmute @user или ответом")]
    Unmute(String),
    #[command(description = "заблокировать пользователя: /ban @user [1d] или ответом")]
    Ban(String),
    #[command(description = "выдать предупреждение: /warn @user [причина] или ответом")]
    Warn(String),
    #[command(description = "снять одно предупреждение: /unwarn @user или ответом")]
    Unwarn(String),
}

fn main() {
    dotenvy::dotenv().ok();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .expect("Не удалось запустить Tokio runtime");

    runtime.block_on(async {
        let client = teloxide::net::default_reqwest_settings()
            .timeout(StdDuration::from_secs(30))
            .connect_timeout(StdDuration::from_secs(15))
            .build()
            .expect("Не удалось создать HTTP-клиент");

        let token =
            std::env::var("TELOXIDE_TOKEN").expect("Переменная TELOXIDE_TOKEN не найдена в .env!");

        let bot = Bot::with_client(token, client);
        let state: SharedState = Arc::new(Mutex::new(BotState::load()));

        loop {
            match bot.get_me().await {
                Ok(me) => {
                    println!("Бот @{} успешно подключился к Telegram.", me.username());
                    break;
                }
                Err(err) => {
                    eprintln!("Ошибка подключения к Telegram API ({err}). Повтор через 3 сек...");
                    tokio::time::sleep(StdDuration::from_secs(3)).await;
                }
            }
        }

        let handler = Update::filter_message()
            .inspect_async(|msg: Message, state: SharedState| async move {
                let mut st = state.lock().await;
                let mut changed = false;
                if let Some(u) = &msg.from {
                    changed |= st.remember_user(u);
                }
                if let Some(reply) = msg.reply_to_message() {
                    if let Some(u) = &reply.from {
                        changed |= st.remember_user(u);
                    }
                }
                if let Some(members) = msg.new_chat_members() {
                    for u in members {
                        changed |= st.remember_user(u);
                    }
                }
                if changed {
                    st.save();
                }
            })
            .branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(handle_command),
            );

        tokio::spawn(async move {
            Dispatcher::builder(bot, handler)
                .dependencies(dptree::deps![state])
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await;
        })
        .await
        .expect("Ошибка в задаче диспетчера");
    });
}

async fn handle_command(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: SharedState,
) -> ResponseResult<()> {
    match cmd {
        Command::Rules => {
            let support_url = Url::parse("https://t.me/YummyAnime_support")
                .expect("Некорректная ссылка поддержки");

            let keyboard = InlineKeyboardMarkup::new(vec![vec![InlineKeyboardButton::url(
                "Поддержка",
                support_url,
            )]]);

            bot.send_message(msg.chat.id, RULES_TEXT)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }

        Command::Mute(args) => {
            if msg.chat.is_private() || !is_sender_admin(&bot, &msg).await? {
                return Ok(());
            }

            let Some((target, rest_args)) = resolve_target(&msg, &args, &state).await else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите пользователя ответом на сообщение или через @username.\nФормат: /mute @username 10m",
                )
                .await?;
                return Ok(());
            };

            if is_user_admin(&bot, msg.chat.id, UserId(target.id)).await {
                bot.send_message(
                    msg.chat.id,
                    "Действие отклонено: невозможно применить ограничение к администратору чата.",
                )
                .await?;
                return Ok(());
            }

            let Some((dur, dur_label)) = extract_duration_from_args(&rest_args) else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите корректную длительность ограничения (например: 1m, 10m, 2h, 1d).",
                )
                .await?;
                return Ok(());
            };

            if dur < Duration::seconds(60) {
                bot.send_message(
                    msg.chat.id,
                    "Минимальный срок ограничения составляет 1 минуту (1m).",
                )
                .await?;
                return Ok(());
            }

            let until = Utc::now() + dur;
            match bot
                .restrict_chat_member(msg.chat.id, UserId(target.id), ChatPermissions::empty())
                .until_date(until)
                .await
            {
                Ok(_) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "🔇 Пользователю {} ограничена отправка сообщений на {}.",
                            target.display_mention(),
                            dur_label
                        ),
                    )
                    .await?;
                }
                Err(err) => {
                    eprintln!("Ошибка при выдаче мута: {err:?}");
                    bot.send_message(
                        msg.chat.id,
                        "Не удалось применить ограничение к указанному пользователю.",
                    )
                    .await?;
                }
            }
        }

        Command::Unmute(args) => {
            if msg.chat.is_private() || !is_sender_admin(&bot, &msg).await? {
                return Ok(());
            }

            let Some((target, _)) = resolve_target(&msg, &args, &state).await else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите пользователя ответом на сообщение или через @username.\nФормат: /unmute @username",
                )
                .await?;
                return Ok(());
            };

            let default_perms = match bot.get_chat(msg.chat.id).await {
                Ok(chat) => chat.permissions().unwrap_or_else(ChatPermissions::all),
                Err(_) => ChatPermissions::all(),
            };

            match bot
                .restrict_chat_member(msg.chat.id, UserId(target.id), default_perms)
                .await
            {
                Ok(_) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "🔊 С пользователя {} сняты ограничения.",
                            target.display_mention()
                        ),
                    )
                    .await?;
                }
                Err(err) => {
                    eprintln!("Ошибка при снятии мута: {err:?}");
                    bot.send_message(
                        msg.chat.id,
                        "Не удалось снять ограничения с указанного пользователя.",
                    )
                    .await?;
                }
            }
        }

        Command::Ban(args) => {
            if msg.chat.is_private() || !is_sender_admin(&bot, &msg).await? {
                return Ok(());
            }

            let Some((target, rest_args)) = resolve_target(&msg, &args, &state).await else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите пользователя ответом на сообщение или через @username.\nФормат: /ban @username или /ban @username 1d",
                )
                .await?;
                return Ok(());
            };

            if is_user_admin(&bot, msg.chat.id, UserId(target.id)).await {
                bot.send_message(
                    msg.chat.id,
                    "Действие отклонено: невозможно заблокировать администратора чата.",
                )
                .await?;
                return Ok(());
            }

            let trimmed = rest_args.trim();
            if trimmed.is_empty() {
                match bot.ban_chat_member(msg.chat.id, UserId(target.id)).await {
                    Ok(_) => {
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🔨 Пользователь {} заблокирован в чате.",
                                target.display_mention()
                            ),
                        )
                        .await?;
                    }
                    Err(err) => {
                        eprintln!("Ошибка при бане: {err:?}");
                        bot.send_message(
                            msg.chat.id,
                            "Не удалось заблокировать указанного пользователя.",
                        )
                        .await?;
                    }
                }
            } else {
                let Some((dur, dur_label)) = extract_duration_from_args(trimmed) else {
                    bot.send_message(
                        msg.chat.id,
                        "Укажите корректную длительность блокировки (например: 10m, 2h, 1d).",
                    )
                    .await?;
                    return Ok(());
                };

                if dur < Duration::seconds(60) {
                    bot.send_message(
                        msg.chat.id,
                        "Минимальный срок временной блокировки составляет 1 минуту (1m).",
                    )
                    .await?;
                    return Ok(());
                }

                let until = Utc::now() + dur;
                match bot
                    .ban_chat_member(msg.chat.id, UserId(target.id))
                    .until_date(until)
                    .await
                {
                    Ok(_) => {
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🔨 Пользователь {} заблокирован в чате на {}.",
                                target.display_mention(),
                                dur_label
                            ),
                        )
                        .await?;
                    }
                    Err(err) => {
                        eprintln!("Ошибка при временном бане: {err:?}");
                        bot.send_message(
                            msg.chat.id,
                            "Не удалось заблокировать указанного пользователя.",
                        )
                        .await?;
                    }
                }
            }
        }

        Command::Warn(args) => {
            if msg.chat.is_private() || !is_sender_admin(&bot, &msg).await? {
                return Ok(());
            }

            let Some((target, reason)) = resolve_target(&msg, &args, &state).await else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите пользователя ответом на сообщение или через @username.\nФормат: /warn @username [причина]",
                )
                .await?;
                return Ok(());
            };

            if is_user_admin(&bot, msg.chat.id, UserId(target.id)).await {
                bot.send_message(
                    msg.chat.id,
                    "Действие отклонено: невозможно выдать предупреждение администратору чата.",
                )
                .await?;
                return Ok(());
            }

            let warn_key = format!("{}:{}", msg.chat.id.0, target.id);
            let count = {
                let mut st = state.lock().await;
                let current = st.warns.get(&warn_key).copied().unwrap_or(0) + 1;
                if current >= 3 {
                    st.warns.remove(&warn_key);
                } else {
                    st.warns.insert(warn_key, current);
                }
                st.save();
                current
            };

            let reason_text = if reason.trim().is_empty() {
                String::new()
            } else {
                format!("\nПричина: {}", reason.trim())
            };

            if count >= 3 {
                let until = Utc::now() + Duration::hours(12);
                match bot
                    .restrict_chat_member(msg.chat.id, UserId(target.id), ChatPermissions::empty())
                    .until_date(until)
                    .await
                {
                    Ok(_) => {
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "⚠️ Пользователю {} выдано 3/3 предупреждений. Назначено автоматическое ограничение отправки сообщений на 12 часов.{}",
                                target.display_mention(),
                                reason_text
                            ),
                        )
                        .await?;
                    }
                    Err(err) => {
                        eprintln!("Ошибка автомута за 3 варна: {err:?}");
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "⚠️ Пользователю {} выдано 3/3 предупреждений. Не удалось применить автоматическое ограничение.{}",
                                target.display_mention(),
                                reason_text
                            ),
                        )
                        .await?;
                    }
                }
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "⚠️ Пользователю {} выдано предупреждение ({}/3).{}",
                        target.display_mention(),
                        count,
                        reason_text
                    ),
                )
                .await?;
            }
        }

        Command::Unwarn(args) => {
            if msg.chat.is_private() || !is_sender_admin(&bot, &msg).await? {
                return Ok(());
            }

            let Some((target, _)) = resolve_target(&msg, &args, &state).await else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите пользователя ответом на сообщение или через @username.\nФормат: /unwarn @username",
                )
                .await?;
                return Ok(());
            };

            let warn_key = format!("{}:{}", msg.chat.id.0, target.id);
            let new_count = {
                let mut st = state.lock().await;
                let current = st.warns.get(&warn_key).copied().unwrap_or(0);
                if current == 0 {
                    0
                } else if current == 1 {
                    st.warns.remove(&warn_key);
                    st.save();
                    0
                } else {
                    let next = current - 1;
                    st.warns.insert(warn_key, next);
                    st.save();
                    next
                }
            };

            bot.send_message(
                msg.chat.id,
                format!(
                    "✅ С пользователя {} снято одно предупреждение. Текущее количество: {}/3.",
                    target.display_mention(),
                    new_count
                ),
            )
            .await?;
        }
    }

    Ok(())
}

async fn is_sender_admin(bot: &Bot, msg: &Message) -> ResponseResult<bool> {
    let Some(user) = &msg.from else {
        return Ok(false);
    };
    let member = bot.get_chat_member(msg.chat.id, user.id).await?;
    Ok(member.is_privileged())
}

async fn is_user_admin(bot: &Bot, chat_id: ChatId, user_id: UserId) -> bool {
    match bot.get_chat_member(chat_id, user_id).await {
        Ok(member) => member.is_privileged(),
        Err(_) => false,
    }
}

async fn resolve_target(
    msg: &Message,
    args: &str,
    state: &SharedState,
) -> Option<(CachedUser, String)> {
    if let Some(reply) = msg.reply_to_message() {
        if let Some(user) = &reply.from {
            return Some((
                CachedUser {
                    id: user.id.0,
                    name: user.first_name.clone(),
                    username: user.username.clone(),
                },
                args.trim().to_string(),
            ));
        }
    }

    if let Some(entities) = msg.entities() {
        for entity in entities {
            if let MessageEntityKind::TextMention { user } = &entity.kind {
                return Some((
                    CachedUser {
                        id: user.id.0,
                        name: user.first_name.clone(),
                        username: user.username.clone(),
                    },
                    args.trim().to_string(),
                ));
            }
        }
    }

    let mut remaining = Vec::new();
    let mut found_user = None;
    let st = state.lock().await;

    for token in args.split_whitespace() {
        if found_user.is_none() {
            if let Some(uname) = token.strip_prefix('@') {
                if let Some(cached) = st.users.get(&uname.to_lowercase()) {
                    let mut entry = cached.clone();
                    if entry.username.is_none() {
                        entry.username = Some(uname.to_string());
                    }
                    found_user = Some(entry);
                    continue;
                }
            } else if let Ok(uid) = token.parse::<u64>() {
                found_user = Some(CachedUser {
                    id: uid,
                    name: format!("ID {uid}"),
                    username: None,
                });
                continue;
            }
        }
        remaining.push(token);
    }

    found_user.map(|u| (u, remaining.join(" ")))
}

fn extract_duration_from_args(args: &str) -> Option<(Duration, String)> {
    for token in args.split_whitespace() {
        if let Some(dur) = parse_duration(token) {
            return Some((dur, token.to_string()));
        }
    }
    None
}

fn parse_duration(input: &str) -> Option<Duration> {
    let s = input.trim();
    let unit = s.chars().last()?;
    let num_str = &s[..s.len() - unit.len_utf8()];
    let num: i64 = num_str.trim().parse().ok()?;
    if num <= 0 {
        return None;
    }
    match unit.to_ascii_lowercase() {
        's' | 'с' => Some(Duration::seconds(num)),
        'm' | 'м' => Some(Duration::minutes(num)),
        'h' | 'ч' => Some(Duration::hours(num)),
        'd' | 'д' => Some(Duration::days(num)),
        _ => None,
    }
}
