use std::collections::HashSet;

use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, text, text_input, toggler, Space,
};
use iced::{Alignment, Element, Length, Task};

use kurozumi_core::torrent::{
    FilterAction, FilterCondition, FilterElement, FilterOperator, FilterState, MatchMode,
    TorrentFeed, TorrentFilter, TorrentItem,
};

use crate::app;
use crate::cover_cache::CoverCache;
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

/// Feed list filter by filter state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedFilter {
    #[default]
    All,
    Matched,
    Preferred,
    Selected,
    Discarded,
}

impl FeedFilter {
    pub const ALL: &[FeedFilter] = &[
        Self::All,
        Self::Matched,
        Self::Preferred,
        Self::Selected,
        Self::Discarded,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Matched => "Matched",
            Self::Preferred => "Preferred",
            Self::Selected => "Selected",
            Self::Discarded => "Discarded",
        }
    }

    fn matches(self, item: &TorrentItem) -> bool {
        match self {
            Self::All => true,
            Self::Matched => item.anime_id.is_some(),
            Self::Preferred => item.filter_state == FilterState::Preferred,
            Self::Selected => item.filter_state == FilterState::Selected,
            Self::Discarded => item.filter_state == FilterState::Discarded,
        }
    }
}

/// Column to sort the feed list by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedSort {
    #[default]
    Title,
    Episode,
    Seeders,
    Size,
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
    // Feed filtering and sorting
    pub feed_filter: FeedFilter,
    pub feed_sort: FeedSort,
    pub sort_ascending: bool,
    // Feed detail panel
    pub selected_torrent: Option<String>,
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
    FeedFilterChanged(FeedFilter),
    FeedSortChanged(FeedSort),
    TorrentSelected(String),
    CloseTorrentDetail,
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
            feed_filter: FeedFilter::default(),
            feed_sort: FeedSort::default(),
            sort_ascending: true,
            selected_torrent: None,
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
                self.selected_torrent = None;
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
            Message::FeedFilterChanged(filter) => {
                self.feed_filter = filter;
                Action::None
            }
            Message::FeedSortChanged(sort) => {
                if self.feed_sort == sort {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.feed_sort = sort;
                    self.sort_ascending = true;
                }
                Action::None
            }
            Message::TorrentSelected(guid) => {
                self.selected_torrent = Some(guid);
                Action::None
            }
            Message::CloseTorrentDetail => {
                self.selected_torrent = None;
                Action::None
            }
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
            async move { db.get_torrent_filters().await.map_err(|e| e.to_string()) },
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
                let filters = db.get_torrent_filters().await.map_err(|e| e.to_string())?;

                // Fetch RSS from all enabled feeds.
                let client = reqwest::Client::new();
                let results = kurozumi_core::torrent::rss::fetch_all_feeds(&client, &feeds).await;
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
                        .archive_torrent(item.guid.clone(), item.title.clone(), "downloaded".into())
                        .await;

                    // Open magnet or torrent link.
                    let link = item.magnet_link.as_deref().or(item.link.as_deref());
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
            async move { db.delete_torrent_feed(id).await.map_err(|e| e.to_string()) },
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

    // ── Sorting & filtering helpers ──────────────────────────────

    /// Build an index of visible feed items after filtering and sorting.
    fn visible_feed_indices(&self) -> Vec<usize> {
        let query_lower = self.search_query.to_lowercase();
        let mut indices: Vec<usize> = self
            .feed_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Text search filter
                if !query_lower.is_empty() && !item.title.to_lowercase().contains(&query_lower) {
                    return false;
                }
                // State filter
                self.feed_filter.matches(item)
            })
            .map(|(i, _)| i)
            .collect();

        // Sort
        let items = &self.feed_items;
        indices.sort_by(|&a, &b| {
            let ia = &items[a];
            let ib = &items[b];
            let ord = match self.feed_sort {
                FeedSort::Title => {
                    let ta = ia.anime_title.as_deref().unwrap_or(&ia.title);
                    let tb = ib.anime_title.as_deref().unwrap_or(&ib.title);
                    ta.to_lowercase().cmp(&tb.to_lowercase())
                }
                FeedSort::Episode => ia.episode.cmp(&ib.episode),
                FeedSort::Seeders => ia.seeders.cmp(&ib.seeders),
                FeedSort::Size => ia.size.cmp(&ib.size),
            };
            if self.sort_ascending {
                ord
            } else {
                ord.reverse()
            }
        });

        indices
    }

    // ── View ─────────────────────────────────────────────────────

    pub fn view<'a>(
        &'a self,
        cs: &ColorScheme,
        cover_cache: &'a CoverCache,
    ) -> Element<'a, Message> {
        let tabs = self.tab_bar(cs);

        let content: Element<'_, Message> = match self.tab {
            TorrentTab::Feed => self.feed_view(cs, cover_cache),
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
                .padding(
                    iced::Padding::new(style::SPACE_XL)
                        .top(style::SPACE_LG)
                        .bottom(style::SPACE_SM),
                ),
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

    fn feed_view<'a>(
        &'a self,
        cs: &ColorScheme,
        cover_cache: &'a CoverCache,
    ) -> Element<'a, Message> {
        // Toolbar: refresh + download + search
        let toolbar = row![
            button(text("Refresh").size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press(Message::RefreshFeeds)
                .style(theme::ghost_button(cs)),
            button(text(format!("Download ({})", self.selected_items.len())).size(style::TEXT_SM))
                .padding([style::SPACE_SM, style::SPACE_LG])
                .on_press_maybe(if self.selected_items.is_empty() {
                    None
                } else {
                    Some(Message::DownloadSelected)
                })
                .style(theme::ghost_button(cs)),
            Space::new().width(Length::Fill),
            text_input("Search torrents...", &self.search_query)
                .on_input(Message::SearchChanged)
                .size(style::TEXT_SM)
                .width(Length::Fixed(200.0)),
        ]
        .spacing(style::SPACE_SM)
        .align_y(Alignment::Center);

        // Filter chips
        let filter_chips = self.feed_filter_chips(cs);

        if self.feed_items.is_empty() {
            let msg = if self.loading {
                "Loading..."
            } else {
                "No torrents. Click Refresh to load RSS feeds."
            };
            return column![
                toolbar,
                filter_chips,
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

        let visible = self.visible_feed_indices();
        let count_text = format!("{} of {} torrents", visible.len(), self.feed_items.len());

        // Sortable column headers
        let header_row = self.feed_column_headers(cs);

        let mut list = column![header_row].spacing(style::SPACE_XXS);

        for idx in &visible {
            let item = &self.feed_items[*idx];
            let is_checked = self.selected_items.contains(&item.guid);
            let is_selected = self.selected_torrent.as_deref() == Some(&item.guid);
            let guid = item.guid.clone();

            let title_display = item.anime_title.as_deref().unwrap_or(&item.title);
            let title_color = if item.anime_id.is_some() {
                cs.primary
            } else {
                match item.filter_state {
                    FilterState::Discarded => cs.outline,
                    FilterState::Selected => cs.on_secondary_container,
                    FilterState::Preferred => cs.tertiary,
                    FilterState::None => cs.on_surface,
                }
            };

            let ep_str = item.episode.map(|e| e.to_string()).unwrap_or_default();
            let group_str = item.release_group.as_deref().unwrap_or("-");
            let size_str = item.size.as_deref().unwrap_or("-");
            let sl_str = match (item.seeders, item.leechers) {
                (Some(s), Some(l)) => format!("{s}/{l}"),
                _ => "-".into(),
            };

            let item_row = button(
                row![
                    checkbox(is_checked).on_toggle({
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
            .on_press(Message::TorrentSelected(item.guid.clone()))
            .padding([style::SPACE_XS, style::SPACE_SM])
            .width(Length::Fill)
            .style(theme::list_item(is_selected, cs));

            list = list.push(item_row);
        }

        let list_content = column![
            toolbar,
            row![
                filter_chips,
                Space::new().width(Length::Fill),
                text(count_text)
                    .size(style::TEXT_XS)
                    .color(cs.outline)
                    .line_height(style::LINE_HEIGHT_LOOSE),
            ]
            .align_y(Alignment::Center),
            crate::widgets::styled_scrollable(list.width(Length::Fill), cs).height(Length::Fill),
        ]
        .spacing(style::SPACE_SM)
        .padding([style::SPACE_SM, style::SPACE_XL])
        .width(Length::Fill)
        .height(Length::Fill);

        // Detail panel
        if let Some(ref guid) = self.selected_torrent {
            if let Some(item) = self.feed_items.iter().find(|i| &i.guid == guid) {
                let detail = torrent_detail_panel(item, cs, cover_cache);
                return row![
                    container(list_content)
                        .width(Length::FillPortion(3))
                        .height(Length::Fill),
                    rule::vertical(1),
                    container(detail)
                        .width(Length::FillPortion(2))
                        .height(Length::Fill),
                ]
                .height(Length::Fill)
                .into();
            }
        }

        list_content.into()
    }

    /// Filter chips row for the Feed tab.
    fn feed_filter_chips(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let chips: Vec<Element<'_, Message>> = FeedFilter::ALL
            .iter()
            .map(|&filter| {
                let is_active = self.feed_filter == filter;
                let mut chip_content = row![].spacing(style::SPACE_XXS).align_y(Alignment::Center);
                if is_active {
                    chip_content =
                        chip_content.push(lucide_icons::iced::icon_check().size(style::TEXT_XS));
                }
                chip_content = chip_content.push(
                    text(filter.label())
                        .size(style::TEXT_XS)
                        .line_height(style::LINE_HEIGHT_LOOSE),
                );

                button(container(chip_content).center_y(Length::Fill))
                    .height(Length::Fixed(style::CHIP_HEIGHT))
                    .padding([style::SPACE_XS, style::SPACE_MD])
                    .on_press(Message::FeedFilterChanged(filter))
                    .style(theme::filter_chip(is_active, cs))
                    .into()
            })
            .collect();

        row(chips).spacing(style::SPACE_XS).into()
    }

    /// Sortable column header row for the Feed tab.
    fn feed_column_headers(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let sort_indicator = |sort: FeedSort| -> String {
            if self.feed_sort == sort {
                if self.sort_ascending {
                    " \u{25B2}".into() // ▲
                } else {
                    " \u{25BC}".into() // ▼
                }
            } else {
                String::new()
            }
        };

        container(
            row![
                // Checkbox spacer
                Space::new().width(Length::Fixed(32.0)),
                // Title header (sortable)
                button(
                    text(format!("Title{}", sort_indicator(FeedSort::Title)))
                        .size(style::TEXT_XS)
                        .color(if self.feed_sort == FeedSort::Title {
                            cs.primary
                        } else {
                            cs.on_surface_variant
                        })
                )
                .on_press(Message::FeedSortChanged(FeedSort::Title))
                .padding(0)
                .style(theme::ghost_button(cs))
                .width(Length::Fill),
                // Episode header (sortable)
                button(
                    text(format!("Ep{}", sort_indicator(FeedSort::Episode)))
                        .size(style::TEXT_XS)
                        .color(if self.feed_sort == FeedSort::Episode {
                            cs.primary
                        } else {
                            cs.on_surface_variant
                        })
                )
                .on_press(Message::FeedSortChanged(FeedSort::Episode))
                .padding(0)
                .style(theme::ghost_button(cs))
                .width(Length::Fixed(40.0)),
                // Group (not sortable)
                text("Group")
                    .size(style::TEXT_XS)
                    .color(cs.on_surface_variant)
                    .width(Length::Fixed(100.0)),
                // Size header (sortable)
                button(
                    text(format!("Size{}", sort_indicator(FeedSort::Size)))
                        .size(style::TEXT_XS)
                        .color(if self.feed_sort == FeedSort::Size {
                            cs.primary
                        } else {
                            cs.on_surface_variant
                        })
                )
                .on_press(Message::FeedSortChanged(FeedSort::Size))
                .padding(0)
                .style(theme::ghost_button(cs))
                .width(Length::Fixed(70.0)),
                // S/L header (sortable)
                button(
                    text(format!("S/L{}", sort_indicator(FeedSort::Seeders)))
                        .size(style::TEXT_XS)
                        .color(if self.feed_sort == FeedSort::Seeders {
                            cs.primary
                        } else {
                            cs.on_surface_variant
                        })
                )
                .on_press(Message::FeedSortChanged(FeedSort::Seeders))
                .padding(0)
                .style(theme::ghost_button(cs))
                .width(Length::Fixed(60.0)),
            ]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
        )
        .padding([style::SPACE_XS, style::SPACE_SM])
        .into()
    }

    // ── Sources tab view ─────────────────────────────────────────

    fn sources_view(&self, cs: &ColorScheme) -> Element<'_, Message> {
        let toolbar = row![button(text("Add Feed").size(style::TEXT_SM))
            .padding([style::SPACE_SM, style::SPACE_LG])
            .on_press(Message::EditFeed(None))
            .style(theme::ghost_button(cs)),]
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
                        Space::new().width(Length::Fill),
                        button(text("Cancel").size(style::TEXT_SM))
                            .padding([style::SPACE_SM, style::SPACE_LG])
                            .on_press(Message::CancelEdit)
                            .style(theme::ghost_button(cs)),
                        button(text("Save").size(style::TEXT_SM))
                            .padding([style::SPACE_SM, style::SPACE_LG])
                            .on_press(Message::SaveFeed)
                            .style(theme::ghost_button(cs)),
                    ]
                    .spacing(style::SPACE_SM),
                ]
                .spacing(style::SPACE_SM),
            )
            .style(theme::card(cs))
            .padding(style::SPACE_LG);
            content = content.push(form);
        }

        // Feed list
        for feed in &self.feeds {
            let feed_id = feed.id;
            let enabled = feed.enabled;

            // "last checked" label
            let checked_label = match &feed.last_checked {
                Some(dt) => crate::format::relative_time(dt),
                None => "never checked".into(),
            };

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
                        text(checked_label)
                            .size(style::TEXT_XS)
                            .color(cs.outline)
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
            .style(theme::card(cs))
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
        let toolbar = row![button(text("Add Filter").size(style::TEXT_SM))
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
            .style(theme::ghost_button(cs)),]
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
                        &[
                            FilterAction::Discard,
                            FilterAction::Select,
                            FilterAction::Prefer
                        ][..],
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
                    button(text("\u{00D7}").size(style::TEXT_SM))
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

            content = content.push(
                container(form)
                    .style(theme::card(cs))
                    .padding(style::SPACE_LG),
            );
        }

        // Filter list
        for filter in &self.filters {
            let filter_id = filter.id;
            let enabled = filter.enabled;
            let action_label = filter.action.to_string();
            let cond_count = format!(
                "{} condition{}",
                filter.conditions.len(),
                if filter.conditions.len() == 1 {
                    ""
                } else {
                    "s"
                }
            );

            // Color-code the action badge
            let action_color = match filter.action {
                FilterAction::Discard => cs.error,
                FilterAction::Select => cs.on_secondary_container,
                FilterAction::Prefer => cs.tertiary,
            };

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
                            container(
                                text(action_label)
                                    .size(style::TEXT_XS)
                                    .color(action_color)
                                    .line_height(style::LINE_HEIGHT_NORMAL),
                            )
                            .style(theme::status_badge(cs, action_color))
                            .padding([style::SPACE_XXS, style::SPACE_SM]),
                            text("\u{00B7}").size(style::TEXT_XS).color(cs.outline),
                            text(cond_count)
                                .size(style::TEXT_XS)
                                .color(cs.on_surface_variant),
                        ]
                        .spacing(style::SPACE_XS)
                        .align_y(Alignment::Center),
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
            .style(theme::card(cs))
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

// ── Torrent detail panel ─────────────────────────────────────────

/// Detail panel for a selected torrent item.
fn torrent_detail_panel<'a>(
    item: &'a TorrentItem,
    cs: &ColorScheme,
    cover_cache: &'a CoverCache,
) -> Element<'a, Message> {
    // Close button
    let close_size = style::TEXT_SM + style::SPACE_XS * 2.0;
    let close_btn = button(
        container(
            lucide_icons::iced::icon_x()
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .on_press(Message::CloseTorrentDetail)
    .padding(0)
    .width(Length::Fixed(close_size))
    .height(Length::Fixed(close_size))
    .style(theme::icon_button(cs));

    let top_bar = row![container("").width(Length::Fill), close_btn].align_y(Alignment::Start);

    let mut detail = column![top_bar].spacing(style::SPACE_MD);

    // Cover image (if matched to an anime)
    if let Some(anime_id) = item.anime_id {
        let cover = crate::widgets::rounded_cover(
            cs,
            cover_cache,
            anime_id,
            style::COVER_WIDTH,
            style::COVER_HEIGHT,
            style::RADIUS_MD,
        );
        detail = detail.push(container(cover).center_x(Length::Fill));
    }

    // Title
    detail = detail.push(
        text(&item.title)
            .size(style::TEXT_BASE)
            .font(style::FONT_HEADING)
            .line_height(style::LINE_HEIGHT_NORMAL),
    );

    // Matched anime name (if different from raw title)
    if let Some(ref anime_title) = item.anime_title {
        detail = detail.push(row![
            text("Matched: ")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant),
            text(anime_title.as_str())
                .size(style::TEXT_SM)
                .color(cs.primary),
        ]);
    }

    // Details card
    let info_label_width = Length::Fixed(90.0);
    let mut info_rows = column![].spacing(style::SPACE_XS);

    let info_row = |label: &'static str, value: String, cs: &ColorScheme| {
        row![
            text(label)
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .width(info_label_width),
            text(value)
                .size(style::TEXT_SM)
                .color(cs.on_surface)
                .line_height(style::LINE_HEIGHT_NORMAL),
        ]
    };

    if let Some(ep) = item.episode {
        info_rows = info_rows.push(info_row("Episode", ep.to_string(), cs));
    }
    if let Some(ref group) = item.release_group {
        info_rows = info_rows.push(info_row("Group", group.clone(), cs));
    }
    if let Some(ref res) = item.resolution {
        info_rows = info_rows.push(info_row("Resolution", res.clone(), cs));
    }
    if let Some(ref size) = item.size {
        info_rows = info_rows.push(info_row("Size", size.clone(), cs));
    }
    if let (Some(s), Some(l)) = (item.seeders, item.leechers) {
        info_rows = info_rows.push(info_row("Seeders", format!("{s} / {l} leechers"), cs));
    }
    if let Some(ref dt) = item.pub_date {
        info_rows = info_rows.push(info_row("Published", crate::format::relative_time(dt), cs));
    }

    // Filter state badge
    let state_label = match item.filter_state {
        FilterState::None => "Unfiltered",
        FilterState::Discarded => "Discarded",
        FilterState::Selected => "Selected",
        FilterState::Preferred => "Preferred",
    };
    let state_color = match item.filter_state {
        FilterState::None => cs.on_surface_variant,
        FilterState::Discarded => cs.error,
        FilterState::Selected => cs.on_secondary_container,
        FilterState::Preferred => cs.tertiary,
    };
    info_rows = info_rows.push(
        row![
            text("Status")
                .size(style::TEXT_SM)
                .color(cs.on_surface_variant)
                .width(info_label_width),
            container(
                text(state_label)
                    .size(style::TEXT_XS)
                    .color(state_color)
                    .line_height(style::LINE_HEIGHT_NORMAL),
            )
            .style(theme::status_badge(cs, state_color))
            .padding([style::SPACE_XXS, style::SPACE_SM]),
        ]
        .align_y(Alignment::Center),
    );

    detail = detail.push(
        container(info_rows)
            .style(theme::card(cs))
            .padding(style::SPACE_MD)
            .width(Length::Fill),
    );

    // Description
    if let Some(ref desc) = item.description {
        if !desc.is_empty() {
            detail = detail.push(
                column![
                    text("Description")
                        .size(style::TEXT_SM)
                        .font(style::FONT_HEADING)
                        .color(cs.on_surface_variant),
                    text(desc.as_str())
                        .size(style::TEXT_SM)
                        .color(cs.on_surface)
                        .line_height(style::LINE_HEIGHT_NORMAL),
                ]
                .spacing(style::SPACE_XS),
            );
        }
    }

    // Download button
    let has_link = item.magnet_link.is_some() || item.link.is_some();
    if has_link {
        let guid = item.guid.clone();
        detail = detail.push(
            button(text("Download").size(style::TEXT_SM).center())
                .padding([style::SPACE_SM, style::SPACE_LG])
                .width(Length::Fill)
                .on_press(Message::ToggleItem(guid))
                .style(theme::ghost_button(cs)),
        );
    }

    crate::widgets::styled_scrollable(
        container(detail)
            .padding(style::SPACE_LG)
            .width(Length::Fill),
        cs,
    )
    .height(Length::Fill)
    .into()
}
