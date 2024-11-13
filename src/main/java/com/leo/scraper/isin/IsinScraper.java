package com.leo.scraper.isin;

import com.leo.scraper.Scraper;
import com.leo.scraper.url.BorsaItaliana;
import com.leo.scraper.selectors.Selectors;

import java.io.IOException;
import java.util.HashMap;
import java.util.Map;

import org.jsoup.nodes.Element;
import org.jsoup.select.Elements;

public class IsinScraper {
  private boolean debug = false;
  private static final int MAX_PAGES = 5;

  private HashMap<String, String> foundIsins = new HashMap<String, String>();

  public void fetchIsins() throws IOException {
    fetchIsins(false);
  }

  public void fetchIsins(boolean debug) throws IOException {
    this.debug = debug;

    scrapeAllLetters();
    IsinFile file = IsinFile.getInstance();

    if (this.debug) {
      System.out.println("\n=====================");
      System.out.println("Scraped " + foundIsins.size() + " ISINs");
    }
    for (Map.Entry<String, String> entry : foundIsins.entrySet()) {
      file.addNewLine(entry.getKey() + " & " + entry.getValue());
    }
  }

  private void scrapeAllLetters() throws IOException {
    for (char letter = 'A'; letter <= 'Z'; letter++) {
      scrapeIsinForLetter(letter);
      if (debug)
        clearPrevLines();
    }
  }

  private void scrapeIsinForLetter(char letter) throws IOException {
    for (int page = 1; page <= MAX_PAGES; page++) {
      scrapeIsinForPage(letter, page);
    }
  }

  private void scrapeIsinForPage(char letter, int page) throws IOException {
    if (this.debug)
      System.out.print("Scraping letter " + letter + " at page " + page + "...");

    Scraper s = Scraper.getInstance(BorsaItaliana.getUrlListinoAzioni(letter, page));

    Elements isins = s.getElements(Selectors.getIsinSelector());

    if (this.debug)
      System.out.println("Found " + isins.size() + " ISINs on page " + page + " of letter " + letter);

    for (Element isin : isins) {
      String[] href = isin.attr("href").split("/");

      String finalIsin = href[href.length - 1].split("\\?")[0].replace(".html", "");
      String share = isin.selectFirst("span.t-text").text();

      foundIsins.put(finalIsin, share);
    }
    if (this.debug)
      System.out.println("Completed scraping for letter '" + letter + "' at page " + page + ".\n");
  }

  private void clearPrevLines() {
    // 11 for each letter
    for (int i = 0; i <= 11; i++) {
      System.out.print("\033[F\033[K");
    }
    System.out.flush();
  }
}
