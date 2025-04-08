from enum import Enum
from typing import List, Optional

class SpyFleeReason(Enum):
    CITY_DESTROYED = "CityDestroyed"
    CITY_CAPTURED = "CityCaptured"
    CITY_BOUGHT = "CityBought"
    CITY_TAKEN_OVER_BY_MARRIAGE = "CityTakenOverByMarriage"
    OTHER = "Other"

class CityEspionageManager:
    def __init__(self):
        self.city = None  # Will be set via set_transients

    def clone(self) -> 'CityEspionageManager':
        return CityEspionageManager()

    def set_transients(self, city):
        self.city = city

    def has_spy_of(self, civ_info) -> bool:
        return any(spy.get_city_or_none() == self.city for spy in civ_info.espionage_manager.spy_list)

    def get_all_stationed_spies(self) -> List['Spy']:
        return [spy for civ in self.city.civ.game_info.civilizations
                for spy in civ.espionage_manager.get_spies_in_city(self.city)]

    def remove_all_present_spies(self, reason: SpyFleeReason):
        for spy in self.get_all_stationed_spies():
            notification_string = {
                SpyFleeReason.CITY_DESTROYED: f"After the city of [{self.city.name}] was destroyed, your spy [{spy.name}] has fled back to our hideout.",
                SpyFleeReason.CITY_CAPTURED: f"After the city of [{self.city.name}] was conquered, your spy [{spy.name}] has fled back to our hideout.",
                SpyFleeReason.CITY_BOUGHT: f"After the city of [{self.city.name}] was taken over, your spy [{spy.name}] has fled back to our hideout.",
                SpyFleeReason.CITY_TAKEN_OVER_BY_MARRIAGE: f"After the city of [{self.city.name}] was taken over, your spy [{spy.name}] has fled back to our hideout.",
                SpyFleeReason.OTHER: f"Due to the chaos ensuing in [{self.city.name}], your spy [{spy.name}] has fled back to our hideout."
            }[reason]

            spy.add_notification(notification_string)
            spy.move_to(None)