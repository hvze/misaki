extern crate discord;
extern crate math_text_transform;
extern crate sharedlib as lib;
extern crate misaki_api;
extern crate glob;
extern crate eval;
extern crate rusqlite;
extern crate rand;
extern crate curl;

use misaki_api::misaki::{MPlugin, MisakiSettings, PluginData};

mod plugins;
mod react;

use discord::{Connection, Discord};
use discord::model::{Message, Event};

use plugins::*;
use react::ReactPlugin; // i hate that big ass plugin.

use std::fs::File;
use std::io::Read;
use std::rc::Rc;

use glob::glob;

use lib::Symbol;
use lib::LibRc;
use lib::FuncRc;

const VERSION: &'static str = "2.1.0 F";

fn read_file(filename: &str) -> String {
    let mut file = File::open(filename).expect(&format!("File \"{}\" not found", filename));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect(&format!(
        "Reading file \"{}\" failed.",
        filename
    ));
    contents.trim().to_string()
}

fn add_default_plugins(mut rc_plugins: &mut Rc<Vec<Box<MPlugin>>>) {
    let plugins = Rc::get_mut(&mut rc_plugins).unwrap();
    plugins.push(Box::new(AboutPlugin));
    plugins.push(Box::new(TextTransformPlugin));
    plugins.push(Box::new(ReactPlugin));
    plugins.push(Box::new(PurgePlugin));
    plugins.push(Box::new(SettingsPlugin));
    plugins.push(Box::new(UserInfoPlugin));
    plugins.push(Box::new(EvalPlugin));
    plugins.push(Box::new(ShillPlugin));
    plugins.push(Box::new(RememberPlugin));
    plugins.push(Box::new(RecallPlugin));
    plugins.push(Box::new(OmnipotencePlugin));
    plugins.push(Box::new(ForgetPlugin));
    plugins.push(Box::new(MockPlugin));
    plugins.push(Box::new(MisconceptionPlugin));
    plugins.push(Box::new(LatexPlugin));
    plugins.push(Box::new(RepeatPlugin));
    plugins.push(Box::new(GamePlugin));
}


fn add_external_plugins(plugins: &mut Vec<(Option<FuncRc<fn() -> Box<MPlugin>>>, Box<MPlugin>)>) {
    for dylib in glob("plugins/compiled/*.dylib").expect("Failed to read glob pattern...") {
        unsafe {
            let lib = LibRc::new(dylib.unwrap()).unwrap();
            let plugin: Box<MPlugin>;
            let get_plugin_ex: lib::FuncRc<_>;
            {
                get_plugin_ex = lib.find_func("get_plugin").unwrap();
                let plugin_sym = get_plugin_ex.get();
                let plugin_ptr: fn() -> Box<MPlugin> = std::mem::transmute(plugin_sym);
                plugin = plugin_ptr();
            }
            plugins.push((Some(get_plugin_ex), plugin));
        }
    }
}

// new function so i don't repeatedly look for a plugin
pub fn execute_plugin_multiple(connection: &Connection, times: i32, plugins: &Rc<Vec<Box<MPlugin>>>, settings: &mut MisakiSettings, discord: &Discord, message: &Message, new_content: String, name: String) {
    let mut f_plugin = None;
    'plugins: for plugin in plugins.iter() {
        'aliases: for alias in plugin.id() {
            if *&name.to_lowercase() == alias {
                discord.delete_message(message.channel_id, message.id).ok();
                f_plugin = Some(plugin);
                break 'plugins;
            }
        }
    }
    discord.delete_message(message.channel_id, message.id).ok();
    if !f_plugin.is_none() {
        (0..times).for_each(|_| {
            discord.delete_message(message.channel_id, message.id).ok();
            let arguments = new_content
                .split_whitespace()
                .skip(1) // skip identifier, thing and MORE
                .map(|x| String::from(x))
                .collect();
            let result = f_plugin.unwrap().execute(PluginData {
                connection,
                discord: &discord,
                message: &message,
                plugins: Rc::clone(plugins),
                arguments,
                settings,
            });
            if !result.is_empty() {
                discord
                    .send_message(
                        message.channel_id,
                        &*format!(
                            "{} {}",
                            if settings.should_mark { "`►`" } else { "" },
                            result
                        ),
                        "",
                        false,
                    )
                    .expect("Failed to send message.");
            }
        });
    }
}

pub fn execute_plugin(connection: &Connection, plugins: &Rc<Vec<Box<MPlugin>>>, settings: &mut MisakiSettings, discord: &Discord, message: &Message, name: String) {
    'plugins: for plugin in plugins.iter() {
        'aliases: for alias in plugin.id() {
            if *&name.to_lowercase() == alias {
                let arguments = message.content
                    .split_whitespace()
                    .skip(1)
                    .map(|x| String::from(x))
                    .collect();
                discord.delete_message(message.channel_id, message.id).ok();
                let result = &*&plugin.execute(PluginData {
                    connection,
                    discord: &discord,
                    message: &message,
                    plugins: Rc::clone(plugins),
                    arguments,
                    settings,
                });
                if !result.is_empty() {
                    discord
                        .send_message(
                            message.channel_id,
                            &*format!(
                                "{} {}",
                                if settings.should_mark { "`►`" } else { "" },
                                result
                            ),
                            "",
                            false,
                        )
                        .expect("Failed to send message.");
                }
                break 'plugins;
            }
        }
    }
}

fn main() {
    let mut plugins: Rc<Vec<Box<MPlugin>>> = Rc::new(Vec::new());
    let mut settings: MisakiSettings = MisakiSettings { react_custom: true, latex_size: 24, ..Default::default() };
    println!("Adding default plugins...");
    add_default_plugins(&mut plugins);
    println!("Added {} plugins!", plugins.len());
    // disable eternal plugins
    // add_external_plugins(&mut plugins);
    println!("Reading token and catalyst...");
    let token = read_file("res/token.txt");
    let catalyst = read_file("res/catalyst.txt");
    println!("Read the token and catalyst successfully!");
    let discord = Discord::from_user_token(&token).expect(&format!("Invalid Token: {}", token));
    let (mut connection, ready) = discord.connect().expect("Connection failed");
    println!("Connected!");
    loop {
        match connection.recv_event() {
            Ok(Event::MessageCreate(ref message)) => if message.author.id == ready.user.id {
                let ref m_content: String = message.content;
                if m_content.chars().take(catalyst.len()).collect::<String>() == catalyst {
                    let ident = m_content
                        .chars()
                        .skip(catalyst.len())
                        .take_while(|&c| c != ' ')
                        .collect::<String>();
                    execute_plugin(&connection, &plugins, &mut settings, &discord, message, ident);
                } else {
                    if settings.uzi_mode {
                        discord.edit_message(message.channel_id, message.id, "");
                    }
                }
            }
            Ok(_) => {}
            Err(discord::Error::Closed(code, body)) => {
                println!("Error: Gateway Closed. Code[{:?}] -- {}", code, body);
                break;
            }
            Err(_) => (),
        }
    }
}