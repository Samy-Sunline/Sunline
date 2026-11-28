#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Home,
    Game,
    Music,
    Advanced,
    Setup,
}

impl Page {
    pub fn initial(setup_done: bool, ready: bool) -> Self {
        if setup_done || ready {
            Self::Home
        } else {
            Self::Setup
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    #[test]
    fn incomplete_first_setup_opens_the_setup_page() {
        assert_eq!(Page::initial(false, false), Page::Setup);
    }

    #[test]
    fn configured_users_go_home_including_older_settings() {
        assert_eq!(Page::initial(false, true), Page::Home);
        assert_eq!(Page::initial(true, true), Page::Home);
        assert_eq!(Page::initial(true, false), Page::Home);
    }
}
