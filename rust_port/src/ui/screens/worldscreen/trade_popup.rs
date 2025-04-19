// Source: orig_src/core/src/com/unciv/ui/screens/worldscreen/TradePopup.kt

use egui::{Color32, RichText, ScrollArea, Ui};
use std::rc::Rc;
use std::cell::RefCell;

use crate::ui::screens::worldscreen::WorldScreen;
use crate::ui::screens::diplomacyscreen::{DiplomacyScreen, LeaderIntroTable};
use crate::logic::trade::{TradeLogic, TradeOffer, TradeOfferType};
use crate::logic::civilization::{NotificationCategory, NotificationIcon};
use crate::ui::popups::Popup;

/// Popup communicating trade offers of others to the player.
///
/// Called in WorldScreen.update, which checks if there are any in viewingCiv.tradeRequests.
pub struct TradePopup {
    world_screen: Rc<RefCell<WorldScreen>>,
    viewing_civ: Rc<RefCell<Civilization>>,
    trade_request: TradeRequest,
}

impl TradePopup {
    pub fn new(world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        let viewing_civ = world_screen.borrow().viewing_civ.clone();
        let trade_request = viewing_civ.borrow().trade_requests.first().unwrap().clone();

        Self {
            world_screen,
            viewing_civ,
            trade_request,
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        let requesting_civ = self.world_screen.borrow().game_info.get_civilization(&self.trade_request.requesting_civ);
        let nation = requesting_civ.nation.clone();
        let trade = self.trade_request.trade.clone();

        let our_resources = self.viewing_civ.borrow().get_civ_resources_by_name();

        // Leader intro table
        let leader_intro = LeaderIntroTable::new(requesting_civ.clone());
        leader_intro.draw(ui);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Trade offers table
        let mut trade_offers_table = egui::Grid::new("trade_offers").striped(true);

        trade_offers_table.header(|ui| {
            ui.label(RichText::new(format!("{}'s trade offer", nation.name)));
            ui.add_space(15.0);
            ui.label(RichText::new("Our trade offer"));
        });

        fn get_offer_text(offer: &TradeOffer, our_resources: &HashMap<String, i32>) -> String {
            let mut trade_text = offer.get_offer_text();
            if offer.offer_type == TradeOfferType::LuxuryResource || offer.offer_type == TradeOfferType::StrategicResource {
                trade_text.push_str(&format!("\nOwned by you: [{}]", our_resources.get(&offer.name).unwrap_or(&0)));
            }
            trade_text
        }

        let max_offers = trade.their_offers.len().max(trade.our_offers.len());
        for i in 0..max_offers {
            trade_offers_table.body(|ui| {
                if i < trade.their_offers.len() {
                    ui.label(get_offer_text(&trade.their_offers[i], &our_resources));
                } else {
                    ui.add_space(1.0);
                }
                ui.add_space(15.0);
                if i < trade.our_offers.len() {
                    ui.label(get_offer_text(&trade.our_offers[i], &our_resources));
                } else {
                    ui.add_space(1.0);
                }
            });
        }

        ScrollArea::vertical().show(ui, |ui| {
            trade_offers_table.ui(ui);
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Play voice line
        // TODO: Implement music controller
        // UncivGame::current().music_controller.play_voice(&format!("{}.tradeRequest", requesting_civ.civ_name));

        ui.label(RichText::new(nation.trade_request));

        if ui.button("Sounds good!").clicked() {
            let trade_logic = TradeLogic::new(self.viewing_civ.clone(), requesting_civ.clone());
            trade_logic.current_trade.set(trade.clone());
            trade_logic.accept_trade();
            self.close();
            TradeThanksPopup::new(leader_intro, self.world_screen.clone()).show(ui);
            requesting_civ.borrow_mut().add_notification(
                &format!("{} has accepted your trade request", self.viewing_civ.borrow().civ_name),
                NotificationCategory::Trade,
                &self.viewing_civ.borrow().civ_name,
                NotificationIcon::Trade,
            );
        }

        if ui.button("Not this time.").clicked() {
            self.trade_request.decline(&self.viewing_civ);
            self.close();
            requesting_civ.borrow_mut().add_notification(
                &format!("{} has denied your trade request", self.viewing_civ.borrow().civ_name),
                NotificationCategory::Trade,
                &this.viewing_civ.borrow().civ_name,
                NotificationIcon::Trade,
            );
            this.world_screen.borrow_mut().should_update = true;
        }

        if ui.button("How about something else...").clicked() {
            this.close();
            this.world_screen.borrow().game.push_screen(DiplomacyScreen::new(
                this.viewing_civ.clone(),
                requesting_civ.clone(),
                trade,
            ));
            this.world_screen.borrow_mut().should_update = true;
        }
    }

    fn close(&self) {
        this.viewing_civ.borrow_mut().trade_requests.remove(&this.trade_request);
    }
}

pub struct TradeThanksPopup {
    world_screen: Rc<RefCell<WorldScreen>>,
    leader_intro: LeaderIntroTable,
}

impl TradeThanksPopup {
    pub fn new(leader_intro: LeaderIntroTable, world_screen: Rc<RefCell<WorldScreen>>) -> Self {
        Self {
            world_screen,
            leader_intro,
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        this.leader_intro.draw(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(15.0);
        ui.label(RichText::new("Excellent!"));

        if ui.button("Farewell.").clicked() {
            this.world_screen.borrow_mut().should_update = true;
        }
    }
}