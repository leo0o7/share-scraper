package com.leo.scraper;

import org.junit.jupiter.api.BeforeAll;
import java.io.IOException;

public abstract class BaseScraperTest {
  protected static Scraper scraper;

  @BeforeAll
  public static void setUp() throws IOException {
    String initialUrl = "https://www.borsaitaliana.it/borsa/azioni/dati-completi.html?isin=IT0003796171&lang=it";
    scraper = Scraper.getInstance(initialUrl);
  }
}
