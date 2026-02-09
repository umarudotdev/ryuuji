use std::collections::HashSet;

use iced::widget::{
    button, checkbox, column, container, pick_list, row, text, text_input, toggler,
    Space,
};
use iced::{Alignment, Element, Length, Task};

use kurozumi_core::torrent::{
    FilterAction, FilterCondition, FilterElement, FilterOperator, MatchMode, TorrentFeed,
    TorrentFilter, TorrentItem,
};

use crate::app;
use crate::db::DbHandle;
use crate::screen::Action;
use crate::style;
use crate::theme::{self, ColorScheme};

/// Which tab is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TorrentTab {
    #[default]
    Feed,
    Filters,
    Sources,
}

/// Torrent screen state.
pub struct Torrents {
    pub tab: TorrentTab,
    pub feed_items: Vec<TorrentItem>,
    pub selected_items: HashSet<String>,
    pub feeds: Vec<TorrentFeed>,
    pub filters: Vec<TorrentFilter>,
    pub loading: bool,
    pub search_query: String,
    // Feed editing
    pub editing_feed: Option<TorrentFeed>,
    pub feed_name_input: String,
    pub feed_url_input: String,
    // Filter editing
    pub editing_filter: Option<TorrentFilter>,
    pub filter_name_input: String,
}

/// Messages handled by the Torrents screen.
#[derive(Debug, Clone)]
pub enum Message {
    TabChanged(TorrentTab),
    SearchChanged(String),
    // Feed tab
    RefreshFeeds,
    FeedItemsLoaded(Result<Vec<TorrentItem>, String>),
    ToggleItem(String),
    DownloadSelected,
    DownloadDone(Result<usize, String>),
    // Sources tab
    FeedsLoaded(Result<Vec<TorrentFeed>, String>),
    ToggleFeed(i64, bool),
    FeedSaved,
    DeleteFeed(i64),
    FeedDeleted,
    EditFeed(Option<TorrentFeed>),
    FeedNameChanged(String),
    FeedUrlChanged(String),
    SaveFeed,
    // Filters tab
    FiltersLoaded(Result<Vec<TorrentFilter>, String>),
    ToggleFilter(i64, bool),
    FilterSaved,
    DeleteFilter(i64),
    FilterDeleted,
    EditFilter(Option<TorrentFilter>),
    FilterNameChanged(String),
    AddCondition,
    RemoveCondition(usize),
    ConditionElementChanged(usize, FilterElement),
    ConditionOperatorChanged(usize, FilterOperator),
    ConditionValueChanged(usize, String),
    FilterMatchModeChanged(MatchMode),
    FilterActionChanged(FilterAction),
    SaveFilter,
    CancelEdit,
}

impl Torrents {
    pub fn new() -> Self {
        Self {
            tab: TorrentTab::default(),
            feed_items: Vec::new(),
            selected_items: HashSet::new(),
            feeds: Vec::new(),
            filters: Vec::new(),
            loading: false,
            search_query: String::new(),
            editing_feed: None,
            feed_name_input: String::new(),
            feed_url_input: String::new(),
            editing_filter: None,
            filter_name_input: String::new(),
        }
    }

    /// Handle a message, returning an Action for the app router.
    pub fn update(&mut self, msg: Message, db: Option<&DbHandle>) -> Action {
        match msg {
            Message::TabChanged(tab) => {
                self.tab = tab;
                self.editing_feed = None;
                self.editing_filter = None;
                match tab {
                    TorrentTab::Sources => return self.load_feeds(db),
                    TorrentTab::Filters => return self.load_filters(db),
                    TorrentTab::Feed => {}
                }
                Action::None
            }
            Message::SearchChanged(q) => {
                self.search_query = q;
                Action::None
            }
            // ── Feed tab ─────────────────────────────────────────
            Message::RefreshFeeds => {
                self.loading = true;
                self.refresh_feeds(db)
            }
            Message::FeedItemsLoaded(Ok(items)) => {
                self.feed_items = items;
                self.loading = false;
                Action::SetStatus(format!("Loaded {} torrents", self.feed_items.len()))
            }
            Message::FeedItemsLoaded(Err(e)) => {
                self.loading = false;
                Action::SetStatus(format!("Feed error: {e}"))
            }
            Message::ToggleItem(guid) => {
                if self.selected_items.contains(&guid) {
                    self.selected_items.remove(&guid);
                } else {
                    self.selected_items.insert(guid);
                }
                Action::None
            }
            Message::DownloadSelected => self.download_selected(db),
            Message::DownloadDone(Ok(count)) => {
                self.selected_items.clear();
                Action::SetStatus(format!("Sent {count} torrents to client"))
            }
            Message::DownloadDone(Err(e)) => Action::SetStatus(format!("Download error: {e}")),
            // ── Sources tab ──────────────────────────────────────
            Message::FeedsLoaded(Ok(feeds)) => {
                self.feeds = feeds;
                Action::None
            }
            Message::FeedsLoaded(Err(e)) => Action::SetStatus(format!("Feed error: {e}")),
            Message::ToggleFeed(id, enabled) => {
                if let Some(feed) = self.feeds.iter_mut().find(|f| f.id == id) {
                    feed.enabled = enabled;
                    let feed = feed.clone();
                    return self.save_feed_action(db, feed);
                }
                Action::None
            }
            Message::FeedSaved => self.load_feeds(db),
            Message::DeleteFeed(id) => self.delete_feed_action(db, id),
            Message::FeedDeleted => self.load_feeds(db),
            Message::EditFeed(feed) => {
                if let Some(ref f) = feed {
                    self.feed_name_input = f.name.clone();
                    self.feed_url_input = f.url.clone();
                } else {
                    self.feed_name_input.clear();
                    self.feed_url_input.clear();
                }
                self.editing_feed = feed;
                Action::None
            }
            Message::FeedNameChanged(s) => {
                self.feed_name_input = s;
                Action::None
            }
            Message::FeedUrlChanged(s) => {
                self.feed_url_input = s;
                Action::None
            }
            Message::SaveFeed => {
                let feed = TorrentFeed {
                    id: self.editing_feed.as_ref().map(|f| f.id).unwrap_or(0),
                    name: self.feed_name_input.trim().to_string(),
                    url: self.feed_url_input.trim().to_string(),
                    enabled: self
                        .editing_feed
                        .as_ref()
                        .map(|f| f.enabled)
                        .unwrap_or(true),
                    last_checked: None,
                };
                self.editing_feed = None;
                self.save_feed_action(db, feed)
            }
            // ── Filters tab ──────────────────────────────────────
            Message::FiltersLoaded(Ok(filters)) => {
                self.filters = filters;
                Action::None
            }
            Message::FiltersLoaded(Err(e)) => Action::SetStatus(format!("Filter error: {e}")),
            Message::ToggleFilter(id, enabled) => {
                if let Some(filter) = self.filters.iter_mut().find(|f| f.id == id) {
                    filter.enabled = enabled;
                    let filter = filter.clone();
                    return self.save_filter_action(db, filter);
                }
                Action::None
            }
            Message::FilterSaved => self.load_filters(db),
            Message::DeleteFilter(id) => self.delete_filter_action(db, id),
            Message::FilterDeleted => self.load_filters(db),
            Message::EditFilter(filter) => {
                if let Some(ref f) = filter {
                    self.filter_name_input = f.name.clone();
                } else {
                    self.filter_name_input.clear();
                }
                self.editing_filter = filter;
                Action::None
            }
            Message::FilterNameChanged(s) => {
                self.filter_name_input = s;
                Action::None
            }
            Message::AddCondition => {
                if let Some(ref mut f) = self.editing_filter {
                    f.conditions.push(FilterCondition {
                        element: FilterElement::Title,
                        operator: FilterOperator::Contains,
                        value: String::new(),
                    });
                }
                Action::None
            }
            Message::RemoveCondition(idx) => {
                if let Some(ref mut f) = self.editing_filter {
                    if idx < f.conditions.len() {
                        f.conditions.remove(idx);
                    }
                }
                Action::None
            }
            Message::ConditionElementChanged(idx, elem) => {
                if let Some(ref mut f) = self.editing_filter {
                    if let Some(c) = f.conditions.get_mut(idx) {
                        c.element = elem;
                    }
                }
                Action::None
            }
            Message::ConditionOperatorChanged(idx, op) => {
                if let Some(ref mut f) = self.editing_filter {
                    if let Some(c) = f.conditions.get_mut(idx) {
                        c.operator = op;
                    }
                }
                Action::None
            }
            Message::ConditionValueChanged(idx, val) => {
                if let Some(ref mut f) = self.editing_filter {
                    if let Some(c) = f.conditions.get_mut(idx) {
                        c.value = val;
                    }
                }
                Action::None
            }
            Message::FilterMatchModeChanged(mode) => {
                if let Some(ref mut f) = self.editing_filter {
                    f.match_mode = mode;
                }
                Action::None
            }
            Message::FilterActionChanged(action) => {
                if let Some(ref mut f) = self.editing_filter {
                    f.action = action;
                }
                Action::None
            }
            Message::SaveFilter => {
                if let Some(mut filter) = self.editing_filter.take() {
                    filter.name = self.filter_name_input.trim().to_string();
                    return self.save_filter_action(db, filter);
                }
                Action::None
            }
            Message::CancelEdit => {
                self.editing_feed = None;
                self.editing_filter = None;
                Action::None
            }
        }
    }

    // ── Async actions ────────────────────────────────────────────

    fn load_feeds(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move { db.get_torrent_feeds().await.map_err(|e| e.to_string()) },
            |r| app::Message::Torrents(Message::FeedsLoaded(r)),
        ))
    }

    fn load_filters(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.get_torrent_filters()
                    .await
                    .map_err(|e| e.to_string())
            },
            |r| app::Message::Torrents(Message::FiltersLoaded(r)),
        ))
    }

    /// Refresh feeds: load feeds → fetch RSS → match → apply filters → filter archived.
    pub fn refresh_feeds(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                // Load feeds and filters from DB.
                let feeds = db.get_torrent_feeds().await.map_err(|e| e.to_string())?;
                let filters = db
                    .get_torrent_filters()
                    .await
                    .map_err(|e| e.to_string())?;

                // Fetch RSS from all enabled feeds.
                let client = reqwest::Client::new();
                let results =
                    kurozumi_core::torrent::rss::fetch_all_feeds(&client, &feeds).await;
                let mut all_items: Vec<TorrentItem> = Vec::new();
                for (_feed_id, result) in results {
                    match result {
                        Ok(items) => all_items.extend(items),
                        Err(e) => tracing::warn!("Feed fetch error: {e}"),
                    }
                }

                // Match titles against library (runs on actor thread).
                let mut all_items = db.match_torrent_items(all_items).await;

                // Apply filters.
                kurozumi_core::torrent::engine::apply_filters(&mut all_items, &filters);

                Ok(all_items)
            },
            |r| app::Message::Torrents(Message::FeedItemsLoaded(r)),
        ))
    }

    fn download_selected(&self, db: Option<&DbHandle>) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        let items: Vec<TorrentItem> = self
            .feed_items
            .iter()
            .filter(|i| self.selected_items.contains(&i.guid))
            .cloned()
            .collect();

        Action::RunTask(Task::perform(
            async move {
                let mut count = 0usize;
                for item in &items {
                    // Archive the item.
                    let _ = db
                        .archive_torrent(
                            item.guid.clone(),
                            item.title.clone(),
                            "downloaded".into(),
                        )
                        .await;

                    // Open magnet or torrent link.
                    let link = item
                        .magnet_link
                        .as_deref()
                        .or(item.link.as_deref());
                    if let Some(url) = link {
                        let _ = open::that(url);
                        count += 1;
                    }
                }
                Ok(count)
            },
            |r| app::Message::Torrents(Message::DownloadDone(r)),
        ))
    }

    fn save_feed_action(&self, db: Option<&DbHandle>, feed: TorrentFeed) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.upsert_torrent_feed(feed)
                    .await
                    .map_err(|e| e.to_string())
            },
            |_| app::Message::Torrents(Message::FeedSaved),
        ))
    }

    fn delete_feed_action(&self, db: Option<&DbHandle>, id: i64) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.delete_torrent_feed(id)
                    .await
                    .map_err(|e| e.to_string())
            },
            |_| app::Message::Torrents(Message::FeedDeleted),
        ))
    }

    fn save_filter_action(&self, db: Option<&DbHandle>, filter: TorrentFilter) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.upsert_torrent_filter(filter)
                    .await
                    .map_err(|e| e.to_string())
            },
            |_| app::Message::Torrents(Message::FilterSaved),
        ))
    }

    fn delete_filter_action(&self, db: Option<&DbHandle>, id: i64) -> Action {
        let Some(db) = db else {
            return Action::None;
        };
        let db = db.clone();
        Action::RunTask(Task::perform(
            async move {
                db.delete_torrent_filter(id)
                    .await
                    .map_err(|e| e.to_string())
            },
            |_| app::Message::Torrents(Message::FilterDeleted),
        ))
    }

    // ── View ─────────────────────────────────────────────────────

    pub fn view(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let tabs = self.tab_bar(cs);

        let content: Element<'_, Message> = match self.tab {
            TorrentTab::Feed => self.feed_view(cs),
            TorrentTab::Filters => self.filter_view(cs),
            TorrentTab::Sources => self.sources_view(cs),
        };

        container(
            column![
                // Header
                container(
                    column![
                        text("Torrents")
                            .size(style::TEXT_XL)
                            .font(style::FONT_HEADING)
                            .line_height(style::LINE_HEIGHT_TIGHT),
                        tabs,
                    ]
                    .spacing(style::SPACE_MD),
                )
                .padding(iced::Padding::new(style::SPACE_XL)
                    .top(style::SPACE_LG)
                    .bottom(style::SPACE_SM)),
                content,
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn tab_bar(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let tab_btn = |label: &'static str, tab: TorrentTab| {
            let active = self.tab == tab;
            button(text(label).size(style::TEXT_SM).center())
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press(Message::TabChanged(tab))
                .style(theme::filter_chip(active, cs))
        };

        row![
            tab_btn("Feed", TorrentTab::Feed),
            tab_btn("Filters", TorrentTab::Filters),
            tab_btn("Sources", TorrentTab::Sources),
        ]
        .spacing(style::SPACE_XS)
        .into()
    }

    // ── Feed tab view ────────────────────────────────────────────

    fn feed_view(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let toolbar = row![
            button(text("Refresh").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press(Message::RefreshFeeds)
                .style(theme::ghost_button(cs)),
            button(
                text(format!("Download ({})", self.selected_items.len()))
                    .size(style::TEXT_SM)
            )
            .padding([style::SPACE_SM, style::SPACE_LG])
            .on_press_maybe(
                if self.selected_items.is_empty() {
                    None
                } else {
                    Some(Message::DownloadSelected)
                }
            )
            .style(theme::ghost_button(cs)),
            Space::new().width(Length::Fill),
            text_input("Search torrents...", &self.search_query)
                .on_input(Message::SearchChanged)
                .size(style::TEXT_SM)
                .width(Length::Fixed(200.0)),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

        if self.feed_items.is_empty() {
            let msg = if self.loading {
                "Loading..."
            } else {
                "No torrents. Click Refresh to load RSS feeds."
            };
            return column![
                toolbar,
                container(
                    text(msg)
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                )
                .center_x(Length::Fill)
                .padding(style::SPACE_3XL),
            ]
            .spacing(style::SPACE_SM)
            .padding([style::SPACE_SM, style::SPACE_XL])
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        let query_lower = self.search_query.to_lowercase();
        let mut list = column![].spacing(style::SPACE_XXS);

        // Header row
        list = list.push(
            container(
                row![
                    Space::new().width(Length::Fixed(32.0)),
                    text("Title")
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fill),
                    text("Ep")
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(40.0)),
                    text("Group")
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(100.0)),
                    text("Size")
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(70.0)),
                    text("S/L")
                        .size(style::TEXT_XS)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(60.0)),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center),
            )
            .padding([style::SPACE_XS, style::SPACE_SM]),
        );

        for item in &self.feed_items {
            // Filter by search query.
            if !query_lower.is_empty() && !item.title.to_lowercase().contains(&query_lower) {
                continue;
            }

            let is_checked = self.selected_items.contains(&item.guid);
            let guid = item.guid.clone();

            let title_display = item
                .anime_title
                .as_deref()
                .unwrap_or(&item.title);
            let title_color = if item.anime_id.is_some() {
                cs.primary
            } else {
                match item.filter_state {
                    kurozumi_core::torrent::FilterState::Discarded => cs.outline,
                    kurozumi_core::torrent::FilterState::Selected => cs.on_secondary_container,
                    kurozumi_core::torrent::FilterState::Preferred => cs.tertiary,
                    kurozumi_core::torrent::FilterState::None => cs.on_surface,
                }
            };

            let ep_str = item
                .episode
                .map(|e| e.to_string())
                .unwrap_or_default();
            let group_str = item.release_group.as_deref().unwrap_or("-");
            let size_str = item.size.as_deref().unwrap_or("-");
            let sl_str = match (item.seeders, item.leechers) {
                (Some(s), Some(l)) => format!("{s}/{l}"),
                _ => "-".into(),
            };

            let item_row = button(
                row![
                    checkbox(is_checked)
                        .on_toggle({
                            let guid = guid.clone();
                            move |_| Message::ToggleItem(guid.clone())
                        }),
                    text(title_display)
                        .size(style::TEXT_SM)
                        .color(title_color)
                        .line_height(style::LINE_HEIGHT_NORMAL)
                        .wrapping(iced::widget::text::Wrapping::None)
                        .width(Length::Fill),
                    text(ep_str)
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(40.0)),
                    text(group_str)
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(100.0)),
                    text(size_str)
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(70.0)),
                    text(sl_str)
                        .size(style::TEXT_SM)
                        .color(cs.on_surface_variant)
                        .width(Length::Fixed(60.0)),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center),
            )
            .on_press(Message::ToggleItem(item.guid.clone()))
            .padding([style::SPACE_XS, style::SPACE_SM])
            .width(Length::Fill)
            .style(theme::list_item(is_checked, cs));

            list = list.push(item_row);
        }

        column![
            toolbar,
            crate::widgets::styled_scrollable(list.width(Length::Fill), cs)
                .height(Length::Fill),
        ]
        .spacing(style::SPACE_SM)
        .padding([style::SPACE_SM, style::SPACE_XL])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    // ── Sources tab view ─────────────────────────────────────────

    fn sources_view(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let toolbar = row![
            button(text("Add Feed").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press(Message::EditFeed(None))
                .style(theme::ghost_button(cs)),
        ]
        .spacing(style::SPACE_SM);

        let mut content = column![toolbar].spacing(style::SPACE_MD);

        // Edit form (if editing)
        if self.editing_feed.is_some() {
            let form = container(
                column![
                    text("Feed Source")
                        .size(style::TEXT_BASE)
                        .font(style::FONT_HEADING),
                    text_input("Feed name", &self.feed_name_input)
                        .on_input(Message::FeedNameChanged)
                        .size(style::TEXT_SM),
                    text_input("RSS URL", &self.feed_url_input)
                        .on_input(Message::FeedUrlChanged)
                        .size(style::TEXT_SM),
                    row![
                        button(text("Save").size(style::TEXT_SM))
                            .padding([style::SPACE_SM, style::SPACE_LG])
                            .on_press(Message::SaveFeed)
                            .style(theme::ghost_button(cs)),
                        button(text("Cancel").size(style::TEXT_SM))
                            .padding([style::SPACE_SM, style::SPACE_LG])
                            .on_press(Message::CancelEdit)
                            .style(theme::ghost_button(cs)),
                    ]
                    .spacing(style::SPACE_SM),
                ]
                .spacing(style::SPACE_SM),
            )
            .padding(style::SPACE_LG);
            content = content.push(form);
        }

        // Feed list
        for feed in &self.feeds {
            let feed_id = feed.id;
            let enabled = feed.enabled;

            let feed_row = container(
                row![
                    toggler(enabled)
                        .on_toggle(move |v| Message::ToggleFeed(feed_id, v))
                        .size(16.0),
                    column![
                        text(&feed.name)
                            .size(style::TEXT_SM)
                            .line_height(style::LINE_HEIGHT_NORMAL),
                        text(&feed.url)
                            .size(style::TEXT_XS)
                            .color(cs.on_surface_variant)
                            .line_height(style::LINE_HEIGHT_LOOSE),
                    ]
                    .spacing(style::SPACE_XXS)
                    .width(Length::Fill),
                    button(text("Edit").size(style::TEXT_XS))
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .on_press(Message::EditFeed(Some(feed.clone())))
                        .style(theme::ghost_button(cs)),
                    button(text("Delete").size(style::TEXT_XS))
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .on_press(Message::DeleteFeed(feed_id))
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_MD)
                .align_y(Alignment::Center),
            )
            .padding([style::SPACE_SM, style::SPACE_MD]);

            content = content.push(feed_row);
        }

        crate::widgets::styled_scrollable(
            content
                .padding([style::SPACE_SM, style::SPACE_XL])
                .width(Length::Fill),
            cs,
        )
        .height(Length::Fill)
        .into()
    }

    // ── Filters tab view ─────────────────────────────────────────

    fn filter_view(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let toolbar = row![
            button(text("Add Filter").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press(Message::EditFilter(Some(TorrentFilter {
                    id: 0,
                    name: String::new(),
                    enabled: true,
                    priority: 0,
                    match_mode: MatchMode::All,
                    action: FilterAction::Select,
                    conditions: vec![],
                })))
                .style(theme::ghost_button(cs)),
        ]
        .spacing(style::SPACE_SM);

        let mut content = column![toolbar].spacing(style::SPACE_MD);

        // Edit form (if editing)
        if let Some(ref filter) = self.editing_filter {
            let mut form = column![
                text("Filter Rule")
                    .size(style::TEXT_BASE)
                    .font(style::FONT_HEADING),
                text_input("Filter name", &self.filter_name_input)
                    .on_input(Message::FilterNameChanged)
                    .size(style::TEXT_SM),
                row![
                    text("Match:").size(style::TEXT_SM),
                    pick_list(
                        &[MatchMode::All, MatchMode::Any][..],
                        Some(filter.match_mode),
                        Message::FilterMatchModeChanged,
                    )
                    .text_size(style::TEXT_SM),
                    text("Action:").size(style::TEXT_SM),
                    pick_list(
                        &[FilterAction::Discard, FilterAction::Select, FilterAction::Prefer][..],
                        Some(filter.action),
                        Message::FilterActionChanged,
                    )
                    .text_size(style::TEXT_SM),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center),
            ]
            .spacing(style::SPACE_SM);

            // Conditions
            for (idx, cond) in filter.conditions.iter().enumerate() {
                let i = idx;
                let cond_row = row![
                    pick_list(
                        &[
                            FilterElement::Title,
                            FilterElement::Episode,
                            FilterElement::ReleaseGroup,
                            FilterElement::Resolution,
                            FilterElement::Size,
                        ][..],
                        Some(cond.element),
                        move |e| Message::ConditionElementChanged(i, e),
                    )
                    .text_size(style::TEXT_SM)
                    .width(Length::Fixed(100.0)),
                    pick_list(
                        &[
                            FilterOperator::Equals,
                            FilterOperator::NotEquals,
                            FilterOperator::Contains,
                            FilterOperator::BeginsWith,
                            FilterOperator::EndsWith,
                            FilterOperator::GreaterThan,
                            FilterOperator::LessThan,
                        ][..],
                        Some(cond.operator),
                        move |o| Message::ConditionOperatorChanged(i, o),
                    )
                    .text_size(style::TEXT_SM)
                    .width(Length::Fixed(120.0)),
                    text_input("value", &cond.value)
                        .on_input(move |v| Message::ConditionValueChanged(i, v))
                        .size(style::TEXT_SM)
                        .width(Length::Fill),
                    button(text("×").size(style::TEXT_SM))
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .on_press(Message::RemoveCondition(idx))
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_SM)
                .align_y(Alignment::Center);
                form = form.push(cond_row);
            }

            form = form.push(
                row![
                    button(text("+ Condition").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_LG])
                        .on_press(Message::AddCondition)
                        .style(theme::ghost_button(cs)),
                    Space::new().width(Length::Fill),
                    button(text("Cancel").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_LG])
                        .on_press(Message::CancelEdit)
                        .style(theme::ghost_button(cs)),
                    button(text("Save").size(style::TEXT_SM))
                        .padding([style::SPACE_SM, style::SPACE_LG])
                        .on_press(Message::SaveFilter)
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_SM),
            );

            content = content.push(container(form).padding(style::SPACE_LG));
        }

        // Filter list
        for filter in &self.filters {
            let filter_id = filter.id;
            let enabled = filter.enabled;
            let action_label = filter.action.to_string();
            let cond_count = format!("{} condition(s)", filter.conditions.len());

            let filter_row = container(
                row![
                    toggler(enabled)
                        .on_toggle(move |v| Message::ToggleFilter(filter_id, v))
                        .size(16.0),
                    column![
                        text(&filter.name)
                            .size(style::TEXT_SM)
                            .line_height(style::LINE_HEIGHT_NORMAL),
                        row![
                            text(action_label)
                                .size(style::TEXT_XS)
                                .color(cs.on_secondary_container),
                            text("·").size(style::TEXT_XS).color(cs.outline),
                            text(cond_count)
                                .size(style::TEXT_XS)
                                .color(cs.on_surface_variant),
                        ]
                        .spacing(style::SPACE_XS),
                    ]
                    .spacing(style::SPACE_XXS)
                    .width(Length::Fill),
                    button(text("Edit").size(style::TEXT_XS))
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .on_press(Message::EditFilter(Some(filter.clone())))
                        .style(theme::ghost_button(cs)),
                    button(text("Delete").size(style::TEXT_XS))
                        .padding([style::SPACE_XS, style::SPACE_SM])
                        .on_press(Message::DeleteFilter(filter_id))
                        .style(theme::ghost_button(cs)),
                ]
                .spacing(style::SPACE_MD)
                .align_y(Alignment::Center),
            )
            .padding([style::SPACE_SM, style::SPACE_MD]);

            content = content.push(filter_row);
        }

        crate::widgets::styled_scrollable(
            content
                .padding([style::SPACE_SM, style::SPACE_XL])
                .width(Length::Fill),
            cs,
        )
        .height(Length::Fill)
        .into()
    }
}
