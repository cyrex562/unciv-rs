#!/usr/bin/env python3
"""
Unciv Python Port - Main Entry Point
"""

import pygame
import sys
import os

# Initialize Pygame
pygame.init()

# Constants
SCREEN_WIDTH = 1024
SCREEN_HEIGHT = 768
FPS = 60
TITLE = "Unciv Python Port"

# Colors
BLACK = (0, 0, 0)
WHITE = (255, 255, 255)
GRAY = (128, 128, 128)
BLUE = (0, 0, 255)
GREEN = (0, 255, 0)
RED = (255, 0, 0)

class Game:
    """Main game class"""

    def __init__(self):
        """Initialize the game"""
        self.screen = pygame.display.set_mode((SCREEN_WIDTH, SCREEN_HEIGHT))
        pygame.display.set_caption(TITLE)
        self.clock = pygame.time.Clock()
        self.running = True
        self.font = pygame.font.SysFont('Arial', 24)

        # Game state
        self.state = "menu"  # menu, game, pause

    def handle_events(self):
        """Handle pygame events"""
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                self.running = False
            elif event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    if self.state == "game":
                        self.state = "pause"
                    elif self.state == "pause":
                        self.state = "game"
                elif event.key == pygame.K_RETURN and self.state == "menu":
                    self.state = "game"

    def update(self):
        """Update game state"""
        if self.state == "game":
            # Update game logic here
            pass
        elif self.state == "pause":
            # Pause menu logic
            pass

    def render(self):
        """Render the game"""
        self.screen.fill(BLACK)

        if self.state == "menu":
            self.render_menu()
        elif self.state == "game":
            self.render_game()
        elif self.state == "pause":
            self.render_pause()

        pygame.display.flip()

    def render_menu(self):
        """Render the main menu"""
        title_text = self.font.render("UNCIV PYTHON PORT", True, WHITE)
        start_text = self.font.render("Press ENTER to start", True, WHITE)

        self.screen.blit(title_text, (SCREEN_WIDTH // 2 - title_text.get_width() // 2, SCREEN_HEIGHT // 3))
        self.screen.blit(start_text, (SCREEN_WIDTH // 2 - start_text.get_width() // 2, SCREEN_HEIGHT // 2))

    def render_game(self):
        """Render the game"""
        # Placeholder for game rendering
        text = self.font.render("Game in progress - Press ESC to pause", True, WHITE)
        self.screen.blit(text, (SCREEN_WIDTH // 2 - text.get_width() // 2, SCREEN_HEIGHT // 2))

    def render_pause(self):
        """Render the pause menu"""
        # First render the game in the background
        self.render_game()

        # Then render the pause overlay
        pause_text = self.font.render("PAUSED - Press ESC to resume", True, WHITE)
        self.screen.blit(pause_text, (SCREEN_WIDTH // 2 - pause_text.get_width() // 2, SCREEN_HEIGHT // 3))

    def run(self):
        """Main game loop"""
        while self.running:
            self.handle_events()
            self.update()
            self.render()
            self.clock.tick(FPS)

        pygame.quit()
        sys.exit()

def main():
    """Main entry point"""
    game = Game()
    game.run()

if __name__ == "__main__":
    main()