#![feature(plugin, decl_macro, custom_derive, proc_macro_non_items)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate lazy_static;
extern crate rand;
extern crate rocket;
extern crate rocket_contrib;
extern crate sha2;

mod game;
mod util;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{self, Form, FlashMessage, FromForm, FormItems, FromRequest, Request};
use rocket::response::{Redirect, Flash, NamedFile};
use rocket_contrib::Template;

use game::{Hand, ParseHandError, Round};
use util::{bytes_to_hex, gen_random_bytes};

// See https://github.com/SergioBenitez/Rocket/issues/693

lazy_static! {
    // User ID -> Session
    static ref SESSIONS: RwLock<HashMap<String, Session>> = RwLock::new(HashMap::new());
}

struct Session {
    user_name: String,
    win_count: usize,
    tie_count: usize,
    loss_count: usize,
    last_round: Option<Round>,
}

impl Session {
    pub fn new(user_name: String) -> Session {
        Session {
            user_name: user_name,
            win_count: 0,
            tie_count: 0,
            loss_count: 0,
            last_round: None,
        }
    }
}

#[derive(FromForm)]
struct Login {
    user_name: String,
}

#[derive(Debug)]
struct User {
    id: String,
    name: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<User, ()> {
        let mut cookies = request.cookies();
        let mut maybe_user = None;
        if let (Some(user_name_ck),
                Some(user_id_ck)) =
               (cookies.get_private("user_name"),
                cookies.get_private("user_id")) {
            let user_id = user_id_ck.value();
            let user_name = user_name_ck.value();
            let sessions = SESSIONS.read().unwrap();
            if let Some(session) = sessions.get(user_id) {
                if session.user_name == user_name {
                    maybe_user = Some(User {
                        id: user_id.to_owned(),
                        name: user_name.to_owned(),
                    });
                }
            }
        }
        maybe_user.or_forward(())
    }
}

struct UserHand(Hand);

impl<'f> FromForm<'f> for UserHand {
    type Error = ParseHandError;

    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<UserHand, ParseHandError> {
        let mut res = Err(ParseHandError);
        for (key, value) in items {
            if key == "hand" {
                res = Ok(UserHand(Hand::from_str(value)?));
                if !strict {
                    return res;
                }
            } else if strict {
                return Err(ParseHandError);
            }
        }
        res
    }
}

fn reset_last_view(context: &mut HashMap<&'static str, String>) {
    const NA: &str = " ∅ ";

    context.insert("win_count", "0".to_owned());
    context.insert("tie_count", "0".to_owned());
    context.insert("loss_count", "0".to_owned());

    context.insert("last_human", NA.to_owned());
    context.insert("last_computer", NA.to_owned());
    context.insert("last_result", NA.to_owned());
    context.insert("last_random", NA.to_owned());
    context.insert("last_hand", NA.to_owned());
    context.insert("last_digest", NA.to_owned());
}

// ===== Routers =====

#[post("/login", data = "<login>")]
fn login(mut cookies: Cookies, login: Form<Login>) -> Result<Redirect, Flash<Redirect>> {
    let user_name = login.get().user_name.to_owned();
    let user_id = bytes_to_hex(&gen_random_bytes(16));
    cookies.add_private(Cookie::new("user_name", user_name.clone()));
    cookies.add_private(Cookie::new("user_id", user_id.clone()));

    let mut sessions = SESSIONS.write().unwrap();
    let session = Session::new(user_name);
    sessions.insert(user_id, session);

    Ok(Redirect::to("/"))
}

#[post("/logout")]
fn logout(mut cookies: Cookies) -> Flash<Redirect> {
    cookies
        .get_private("user_id")
        .map(|cookie| {
            let user_id = cookie.value();
            let mut sessions = SESSIONS.write().unwrap();
            sessions.remove(user_id);
        });

    cookies.remove_private(Cookie::named("user_name"));
    cookies.remove_private(Cookie::named("user_id"));

    Flash::success(Redirect::to("/login"), "Successfully logged out.")
}

#[get("/login")]
fn login_user(_user: User) -> Redirect {
    Redirect::to("/")
}

#[get("/login", rank = 2)]
fn login_page(flash: Option<FlashMessage>) -> Template {
    let mut context = HashMap::new();
    if let Some(ref msg) = flash {
        context.insert("flash", msg.msg());
    }

    Template::render("login", &context)
}


#[get("/", rank = 1)]
fn user_index(user: User) -> Template {
    let mut context = HashMap::new();
    context.insert("user_name", user.name.clone());
    reset_last_view(&mut context);

    let round = Round::random();
    context.insert("digest", round.digest.clone());
    let mut sessions = SESSIONS.write().unwrap();
    sessions
        .get_mut(&user.id)
        .map(|session| session.last_round = Some(round));

    Template::render("index", &context)
}

#[get("/", rank = 2)]
fn index() -> Redirect {
    Redirect::to("/login")
}

#[get("/?<hand>")]
fn user_play_index(user: User, hand: UserHand) -> Template {
    let mut context = HashMap::new();
    context.insert("user_id", user.name.clone());

    let mut sessions = SESSIONS.write().unwrap();
    match sessions.get_mut(&user.id) {
        Some(ref mut session) => {
            // Reports the result of the last round.
            let last_round = session.last_round.as_mut().expect(
                "`last_round` should have been initialized in `user_index`.");
            let result = match last_round.computer.vs(&hand.0) {
                Ordering::Greater => {
                    session.loss_count += 1;
                    "Computer won"
                }
                Ordering::Equal => {
                    session.tie_count += 1;
                    "Tie"
                }
                Ordering::Less => {
                    session.win_count += 1;
                    "You won"
                }
            };

            context.insert("win_count", format!("{}", session.win_count));
            context.insert("tie_count", format!("{}", session.tie_count));
            context.insert("loss_count", format!("{}", session.loss_count));

            context.insert("last_human", hand.0.as_icon().to_owned());
            context.insert("last_computer", last_round.computer.as_icon().to_owned());
            context.insert("last_result", result.to_owned());
            context.insert("last_random", last_round.random_bytes.to_owned());
            context.insert("last_hand", last_round.computer.as_ref().to_owned());
            context.insert("last_digest", last_round.digest.to_owned());

            // Starts a new round
            let round = Round::random();
            context.insert("digest", round.digest.clone());
            *last_round = round;
        }
        _ => {
            reset_last_view(&mut context);
        }
    }
    Template::render("index", &context)
}

#[get("/<file..>", rank = 10)]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .attach(Template::fairing())
        .mount("/",
            routes![
                index, user_index, user_play_index,
                login, logout, login_user, login_page,
                files
            ])
}

fn main() {
    rocket().launch();
}
