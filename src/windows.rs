use xcb::{self, Connection, Screen, Window, Atom};
use std::sync::Mutex;
use std::collections::HashMap;
use encoding::{Encoding, DecoderTrap};
use encoding::all::ISO_8859_1;
use conditions::Condition;

const XCB_EWMH_CLIENT_SOURCE_TYPE_OTHER: u32 = 2;

lazy_static! {
    static ref INTERNED_ATOMS: Mutex<HashMap<&'static str, Atom>> = Mutex::new(HashMap::new());
}

pub struct WindowTreeIter<'a> {
    pub conn: &'a Connection,
    pub stack: Vec<Window>,
}

impl<'a> WindowTreeIter<'a> {
    fn new(conn: &'a Connection, win: Window) -> Result<WindowTreeIter<'a>, xcb::GenericError> {
        let reply = xcb::query_tree(conn, win).get_reply()?;
        Ok(WindowTreeIter {
            conn: conn,
            stack: reply.children().to_owned(),
        })
    }
}

impl<'a> Iterator for WindowTreeIter<'a> {
    type Item = Result<Window, xcb::GenericError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop().map(|top| {
            xcb::query_tree(self.conn, top).get_reply().map(|reply| {
                self.stack.extend(reply.children());
                top
            })
        })
    }
}

pub fn get_atom(conn: &Connection, atom: &'static str) -> Atom {
    let current = {
        INTERNED_ATOMS.lock().unwrap().get(atom).cloned()
    };
    current.unwrap_or_else(|| {
        let interned = xcb::intern_atom(conn, true, atom).get_reply().unwrap().atom();
        INTERNED_ATOMS.lock().unwrap().insert(atom, interned);
        interned
    })
}

pub fn set_active_window(conn: &Connection, screen: &Screen, win: Window) {
    let net_active_window = get_atom(conn, "_NET_ACTIVE_WINDOW");
    let data = xcb::ClientMessageData::from_data32([XCB_EWMH_CLIENT_SOURCE_TYPE_OTHER,
                                                    xcb::CURRENT_TIME,
                                                    xcb::WINDOW_NONE,
                                                    0,
                                                    0]);
    let ev = xcb::ClientMessageEvent::new(32, win, net_active_window, data);
    xcb::send_event(conn,
                    false,
                    screen.root(),
                    xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT,
                    &ev);
}

pub fn get_string_property(conn: &Connection, window: Window, prop: Atom) -> Option<String> {
    let reply = match xcb::get_property(conn,
                                        false,
                                        window,
                                        prop,
                                        xcb::GET_PROPERTY_TYPE_ANY,
                                        0,
                                        u32::max_value())
                          .get_reply() {
        Ok(r) => r,
        _ => return None,
    };
    let atom_utf8_string = get_atom(conn, "UTF8_STRING");
    let reply_type = reply.type_();
    if reply_type == xcb::ATOM_STRING {
        ISO_8859_1.decode(reply.value(), DecoderTrap::Strict).ok()
    } else if reply_type == atom_utf8_string {
        String::from_utf8(reply.value().to_vec()).ok()
    } else {
        None
    }
}

pub fn is_regular_window(conn: &Connection, window: Window) -> bool {
    let atom_wm_state = get_atom(conn, "WM_STATE");
    xcb::get_property(conn,
                      false,
                      window,
                      atom_wm_state,
                      atom_wm_state,
                      0,
                      u32::max_value())
        .get_reply()
        .map(|state| state.value_len() > 0)
        .unwrap_or(false)
}

pub fn find_matching_window(conn: &Connection,
                            screen: &Screen,
                            cond: &Condition)
                            -> Result<Option<Window>, xcb::GenericError> {
    let wins = WindowTreeIter::new(&conn, screen.root())?;
    for w in wins {
        let w = w?;
        if is_regular_window(conn, w) && cond.matches(conn, w) {
            return Ok(Some(w));
        }
    }
    Ok(None)
}
