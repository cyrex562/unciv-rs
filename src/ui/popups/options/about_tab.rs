use ggez::graphics::{DrawParam, Text};
use ggez::mint::Point2;
use ggez::{Context, GameResult};

use crate::constants::Constants;
use crate::game::UncivGame;
use crate::ui::screens::civilopedia_screen::{FormattedLine, MarkupRenderer};

pub struct AboutTab;

impl AboutTab {
    pub fn render(ctx: &mut Context, game: &UncivGame) -> GameResult<()> {
        let version_anchor = game.version.text.replace(".", "");

        // Create formatted lines for the about tab
        let mut lines = Vec::new();

        // Add banner image
        lines.push(FormattedLine::new_with_image("banner", 240.0, true));
        lines.push(FormattedLine::new_empty());

        // Add version with link to changelog
        let version_text = format!("Version: {}", game.version.to_nice_string());
        let changelog_link = format!("{}/blob/master/changelog.md#{}",
            Constants::UNCIV_REPO_URL, version_anchor);
        lines.push(FormattedLine::new_with_link(&version_text, &changelog_link));

        // Add readme link
        let readme_link = format!("{}/blob/master/README.md#unciv---foss-civ-v-for-androiddesktop",
            Constants::UNCIV_REPO_URL);
        lines.push(FormattedLine::new_with_link("See online Readme", &readme_link));

        // Add repository link
        lines.push(FormattedLine::new_with_link("Visit repository", Constants::UNCIV_REPO_URL));

        // Add wiki link
        lines.push(FormattedLine::new_with_link("Visit the wiki", Constants::WIKI_URL));

        // Render the formatted lines
        let rendered_content = MarkupRenderer::render(&lines);

        // Position the content with padding
        let padding = 20.0;
        let position = Point2 { x: padding, y: padding };

        // Draw the rendered content
        rendered_content.draw(ctx, DrawParam::default().dest(position))?;

        Ok(())
    }
}