from typing import List, Optional, Dict, Set, Tuple
from dataclasses import dataclass
import math

from com.unciv.logic.civilization import Civilization
from com.unciv.logic.civilization import NotificationCategory, NotificationIcon
from com.unciv.logic.civilization.diplomacy import DiplomacyFlags, RelationshipLevel
from com.unciv.logic.trade import (
    Trade, TradeEvaluation, TradeLogic, TradeOffer,
    TradeRequest, TradeOfferType
)

class TradeAutomation:
    """Handles AI automation for trade decisions and actions."""

    @staticmethod
    def respond_to_trade_requests(civ_info: Civilization, trade_and_change_state: bool) -> None:
        """Respond to incoming trade requests.
        
        Args:
            civ_info: The civilization responding to requests
            trade_and_change_state: Whether to update trade state
        """
        for trade_request in list(civ_info.trade_requests):
            other_civ = civ_info.game_info.get_civilization(trade_request.requesting_civ)
            # Treat 'no trade' state as if all trades are invalid - thus AIs will not update its "turns to offer"
            if not trade_and_change_state or not TradeEvaluation().is_trade_valid(
                trade_request.trade, civ_info, other_civ
            ):
                continue

            trade_logic = TradeLogic(civ_info, other_civ)
            trade_logic.current_trade.set(trade_request.trade)
            # We need to remove this here, so that if the trade is accepted, the updateDetailedCivResources()
            # in tradeLogic.acceptTrade() will not consider *both* the trade *and the trade offer as decreasing the
            # amount of available resources, since that will lead to "Our proposed trade is no longer valid" if we try to offer
            # the same resource to ANOTHER civ in this turn. Complicated!
            civ_info.trade_requests.remove(trade_request)
            
            if TradeEvaluation().is_trade_acceptable(trade_logic.current_trade, civ_info, other_civ):
                trade_logic.accept_trade()
                other_civ.add_notification(
                    f"[{civ_info.civ_name}] has accepted your trade request",
                    NotificationCategory.Trade,
                    NotificationIcon.Trade,
                    civ_info.civ_name
                )
            else:
                counteroffer = TradeAutomation._get_counteroffer(civ_info, trade_request)
                if counteroffer is not None:
                    other_civ.add_notification(
                        f"[{civ_info.civ_name}] has made a counteroffer to your trade request",
                        NotificationCategory.Trade,
                        NotificationIcon.Trade,
                        civ_info.civ_name
                    )
                    other_civ.trade_requests.add(counteroffer)
                else:
                    other_civ.add_notification(
                        f"[{civ_info.civ_name}] has denied your trade request",
                        NotificationCategory.Trade,
                        civ_info.civ_name,
                        NotificationIcon.Trade
                    )
                    trade_request.decline(civ_info)
                    
        civ_info.trade_requests.clear()

    @staticmethod
    def _get_counteroffer(civ_info: Civilization, trade_request: TradeRequest) -> Optional[TradeRequest]:
        """Generate a counteroffer for a trade request.
        
        Args:
            civ_info: The civilization making the counteroffer
            trade_request: The original trade request
            
        Returns:
            Optional[TradeRequest]: The counteroffer, if any
        """
        other_civ = civ_info.game_info.get_civilization(trade_request.requesting_civ)
        # AIs counteroffering each other could be problematic if they ping-pong back and forth forever
        # If this happens, that means our trade automation doesn't settle into an equilibrium that's favourable to both parties, so that should be updated when observed
        evaluation = TradeEvaluation()
        delta_in_our_favor = evaluation.get_trade_acceptability(
            trade_request.trade, civ_info, other_civ, True
        )
        if delta_in_our_favor > 0:
            delta_in_our_favor = int(delta_in_our_favor / 1.1)  # They seem very interested in this deal, let's push it a bit.
            
        trade_logic = TradeLogic(civ_info, other_civ)
        trade_logic.current_trade.set(trade_request.trade)

        # What do they have that we would want?
        potential_asks: Dict[TradeOffer, int] = {}
        counteroffer_asks: Dict[TradeOffer, int] = {}
        counteroffer_gifts: List[TradeOffer] = []

        for offer in trade_logic.their_available_offers:
            if ((offer.type == TradeOfferType.Gold or offer.type == TradeOfferType.Gold_Per_Turn)
                and any(offer.type == our_offer.type for our_offer in trade_request.trade.our_offers)):
                continue  # Don't want to counteroffer straight gold for gold, that's silly
            if not offer.is_tradable():
                continue  # For example resources gained by trade or CS
            if offer.type == TradeOfferType.City:
                continue  # Players generally don't want to give up their cities, and they might misclick
            if offer.type == TradeOfferType.Luxury_Resource:
                continue  # Don't ask for luxuries as counteroffer, players likely don't want to sell them if they didn't offer them already
            if any(offer.type == their_offer.type and offer.name == their_offer.name 
                  for their_offer in trade_logic.current_trade.their_offers):
                continue  # So you don't get double offers of open borders declarations of war etc.
            if offer.type == TradeOfferType.Treaty:
                continue  # Don't try to counter with a defensive pact or research pact

            value = evaluation.evaluate_buy_cost_with_inflation(
                offer, civ_info, other_civ, trade_request.trade
            )
            if value > 0:
                potential_asks[offer] = value

        while potential_asks and delta_in_our_favor < 0:
            # Keep adding their worst offer until we get above the threshold
            offer_to_add = min(potential_asks.items(), key=lambda x: x[1])
            delta_in_our_favor += offer_to_add[1]
            counteroffer_asks[offer_to_add[0]] = offer_to_add[1]
            del potential_asks[offer_to_add[0]]

        if delta_in_our_favor < 0:
            return None  # We couldn't get a good enough deal

        # At this point we are sure to find a good counteroffer
        while delta_in_our_favor > 0:
            # Now remove the best offer valued below delta until the deal is barely acceptable
            offers_to_remove = [
                (offer, value) for offer, value in counteroffer_asks.items()
                if value <= delta_in_our_favor
            ]
            if not offers_to_remove:
                break  # Nothing more can be removed, at least en bloc
                
            offer_to_remove = max(offers_to_remove, key=lambda x: x[1])
            delta_in_our_favor -= offer_to_remove[1]
            del counteroffer_asks[offer_to_remove[0]]

        # Only ask for enough of each resource to get maximum price
        for ask in [offer for offer in counteroffer_asks.keys()
                   if offer.type in (TradeOfferType.Luxury_Resource, TradeOfferType.Strategic_Resource)]:
            # Remove 1 amount as long as doing so does not change the price
            original_value = counteroffer_asks[ask]
            while (ask.amount > 1
                   and original_value == evaluation.evaluate_buy_cost_with_inflation(
                       TradeOffer(ask.name, ask.type, ask.amount - 1, ask.duration),
                       civ_info, other_civ, trade_request.trade)):
                ask.amount -= 1

        # Adjust any gold asked for
        to_remove: List[TradeOffer] = []
        for gold_ask in sorted(
            [offer for offer in counteroffer_asks.keys()
             if offer.type in (TradeOfferType.Gold_Per_Turn, TradeOfferType.Gold)],
            key=lambda x: x.type.ordinal,
            reverse=True  # Do GPT first
        ):
            value_of_one = evaluation.evaluate_buy_cost_with_inflation(
                TradeOffer(gold_ask.name, gold_ask.type, 1, gold_ask.duration),
                civ_info, other_civ, trade_request.trade
            )
            amount_can_be_removed = delta_in_our_favor / value_of_one
            if amount_can_be_removed >= gold_ask.amount:
                delta_in_our_favor -= counteroffer_asks[gold_ask]
                to_remove.append(gold_ask)
            else:
                delta_in_our_favor -= value_of_one * amount_can_be_removed
                gold_ask.amount -= amount_can_be_removed

        # If the delta is still very in our favor consider sweetening the pot with some gold
        if delta_in_our_favor >= 100:
            delta_in_our_favor = (delta_in_our_favor * 2) // 3  # Only compensate some of it though, they're the ones asking us
            # First give some GPT, then lump sum - but only if they're not already offering the same
            for our_gold in sorted(
                [offer for offer in trade_logic.our_available_offers
                 if (offer.is_tradable() 
                     and offer.type in (TradeOfferType.Gold, TradeOfferType.Gold_Per_Turn))],
                key=lambda x: x.type.ordinal,
                reverse=True
            ):
                if (not any(offer.type == our_gold.type 
                           for offer in trade_logic.current_trade.their_offers)
                    and not any(offer.type == our_gold.type 
                              for offer in counteroffer_asks.keys())):
                    value_of_one = evaluation.evaluate_sell_cost_with_inflation(
                        TradeOffer(our_gold.name, our_gold.type, 1, our_gold.duration),
                        civ_info, other_civ, trade_request.trade
                    )
                    amount_to_give = min(delta_in_our_favor / value_of_one, our_gold.amount)
                    delta_in_our_favor -= amount_to_give * value_of_one
                    if amount_to_give > 0:
                        counteroffer_gifts.append(
                            TradeOffer(
                                our_gold.name,
                                our_gold.type,
                                amount_to_give,
                                our_gold.duration
                            )
                        )

        trade_logic.current_trade.their_offers.extend(counteroffer_asks.keys())
        trade_logic.current_trade.our_offers.extend(counteroffer_gifts)

        # Trades reversed, because when *they* get it then the 'ouroffers' become 'theiroffers'
        return TradeRequest(civ_info.civ_name, trade_logic.current_trade.reverse())

    @staticmethod
    def exchange_luxuries(civ_info: Civilization) -> None:
        """Exchange luxury resources with other civilizations.
        
        Args:
            civ_info: The civilization exchanging luxuries
        """
        known_civs = civ_info.get_known_civs()

        # Player trades are... more complicated.
        # When the AI offers a trade, it's not immediately accepted,
        # so what if it thinks that it has a spare luxury and offers it to two human players?
        # What's to stop the AI "nagging" the player to accept a luxury trade?
        # We should A. add some sort of timer (20? 30 turns?) between luxury trade requests if they're denied - see DeclinedLuxExchange
        # B. have a way for the AI to keep track of the "pending offers" - see DiplomacyManager.resourcesFromTrade

        for other_civ in [civ for civ in known_civs
                         if (civ.is_major_civ()
                             and not civ.is_at_war_with(civ_info)
                             and not civ_info.get_diplomacy_manager(civ).has_flag(DiplomacyFlags.DeclinedLuxExchange))]:
            is_enemy = civ_info.get_diplomacy_manager(other_civ).is_relationship_level_le(RelationshipLevel.Enemy)
            if (is_enemy 
                or any(request.requesting_civ == civ_info.civ_name 
                      for request in other_civ.trade_requests)):
                continue

            trades = TradeAutomation._potential_luxury_trades(civ_info, other_civ)
            for trade in trades:
                trade_request = TradeRequest(civ_info.civ_name, trade.reverse())
                other_civ.trade_requests.add(trade_request)

    @staticmethod
    def _potential_luxury_trades(civ_info: Civilization, other_civ_info: Civilization) -> List[Trade]:
        """Find potential luxury trades between two civilizations.
        
        Args:
            civ_info: The first civilization
            other_civ_info: The second civilization
            
        Returns:
            List[Trade]: List of potential trades
        """
        trade_logic = TradeLogic(civ_info, other_civ_info)
        our_tradable_luxury_resources = [
            offer for offer in trade_logic.our_available_offers
            if offer.type == TradeOfferType.Luxury_Resource and offer.amount > 1
        ]
        their_tradable_luxury_resources = [
            offer for offer in trade_logic.their_available_offers
            if offer.type == TradeOfferType.Luxury_Resource and offer.amount > 1
        ]
        
        we_have_they_dont = [
            resource for resource in our_tradable_luxury_resources
            if not any(offer.name == resource.name 
                      and offer.type == TradeOfferType.Luxury_Resource
                      for offer in trade_logic.their_available_offers)
        ]
        
        they_have_we_dont = sorted(
            [resource for resource in their_tradable_luxury_resources
             if not any(offer.name == resource.name 
                       and offer.type == TradeOfferType.Luxury_Resource
                       for offer in trade_logic.our_available_offers)],
            key=lambda x: sum(1 for city in civ_info.cities 
                            if city.demanded_resource == x.name)  # Prioritize resources that get WLTKD
        )
        
        trades = []
        for i in range(min(len(we_have_they_dont), len(they_have_we_dont))):
            trade = Trade()
            trade.our_offers.append(we_have_they_dont[i].copy(amount=1))
            trade.their_offers.append(they_have_we_dont[i].copy(amount=1))
            trades.append(trade)
            
        return trades 