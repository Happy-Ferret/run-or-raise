use regex::Regex;
use xcb::{self, Connection, Window};
use windows::{get_string_property, get_atom};

#[derive(Debug,PartialEq)]
pub enum Property {
    Class,
    Name,
    Role,
}

#[derive(Debug)]
pub struct Match {
    pub prop: Property,
    pub op: Operator,
}

#[derive(Debug)]
pub enum Operator {
    Regex(Regex),
    Equal(String),
}

#[derive(Debug)]
pub enum Condition {
    Pure(Match),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

impl Property {
    pub fn from_window(&self, conn: &Connection, win: Window) -> Option<String> {
        match *self {
            Property::Class => {
                get_string_property(conn, win, xcb::ATOM_WM_CLASS)
                    .map(|p| p.split('\u{0}').nth(1).unwrap().to_owned())
            }
            Property::Name => {
                get_string_property(conn, win, get_atom(conn, "_NET_WM_NAME"))
                    .or(get_string_property(conn, win, xcb::ATOM_WM_NAME))
            }
            Property::Role => get_string_property(conn, win, get_atom(conn, "WM_WINDOW_ROLE")),
        }
    }
}

impl Match {
    pub fn matches(&self, conn: &Connection, win: Window) -> bool {
        self.prop
            .from_window(conn, win)
            .map(|p| {
                match self.op {
                    Operator::Regex(ref pattern) => pattern.is_match(&p),
                    Operator::Equal(ref value) => value == &p,
                }
            })
            .unwrap_or(false)
    }
}

// TODO: Avoid multiple lookups
impl Condition {
    pub fn matches(&self, conn: &Connection, win: Window) -> bool {
        match *self {
            Condition::Pure(ref m) => m.matches(conn, win),
            Condition::And(ref a, ref b) => a.matches(conn, win) && b.matches(conn, win),
            Condition::Or(ref a, ref b) => a.matches(conn, win) || b.matches(conn, win),
            Condition::Not(ref a) => !a.matches(conn, win),
        }
    }
}
