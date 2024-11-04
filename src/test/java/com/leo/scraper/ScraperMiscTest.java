package com.leo.scraper;

import static org.junit.jupiter.api.Assertions.*;

import java.io.File;
import java.io.IOException;

import org.junit.jupiter.api.*;

public class ScraperMiscTest extends BaseScraperTest {
  // basic functionality
  @Test
  public void testScraperSingletonInitialization() {
    assertNotNull(scraper, "Scraper instance should be initialized");
  }

  @Test
  public void testLogging() {
    File logFile = new File("scraper.log");
    assertTrue(logFile.exists(), "Log file should exist after initialization");
  }

  @Test
  public void testUpdateScrapeUrl() throws IOException {
    scraper.updateScrapeUrl("https://www.new-url.com");
    assertEquals("https://www.new-url.com", scraper.getScrapeUrl(),
        "Scrape url should update when calling updateScrapeUrl");
  }
}
