pub fn remove_missing_mod_references(game_info: &mut GameInfo) {
    game_info
        .tile_map
        .remove_missing_terrain_mod_references(&game_info.ruleset);

    remove_units_and_promotions(game_info);
    remove_missing_great_person_points(game_info);

    // Mod decided you can't repair things anymore - get rid of old pillaged improvements
    remove_old_pillaged_improvements(game_info);
    remove_missing_last_seen_improvements(game_info);

    handle_missing_references_for_each_city(game_info);

    remove_tech_and_policies(game_info);
}
