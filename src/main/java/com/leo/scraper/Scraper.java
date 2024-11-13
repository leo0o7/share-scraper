package com.leo.scraper;

import org.jsoup.Jsoup;
import org.jsoup.nodes.Document;
import org.jsoup.nodes.Element;
import org.jsoup.select.Elements;

import java.io.BufferedWriter;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.concurrent.Callable;
import java.util.function.Function;
import java.util.regex.Pattern;

public class Scraper {
  // allow both HH and H hours with H
  private static final DateTimeFormatter DATE_TIME_FORMATTER = DateTimeFormatter.ofPattern("dd/MM/yy - H.mm.ss");
  private static final DateTimeFormatter DATE_FORMATTER = DateTimeFormatter.ofPattern("dd/MM/yy");
  // singleton instance of the class
  private static Scraper scraper;
  // logging info and errors
  private static final File LOG_FILE = new File("scraper.log");
  // timeout till document unloads
  // that means processing should be at most 60s
  // private static final int TIMEOUT = 60 * 1000;
  private Document doc;
  private String scrapeUrl;
  // exponential backoff methods
  private Callable<Document> CONNECT_TO_PAGE = () -> Jsoup.connect(scrapeUrl)
      // .userAgent(
      // "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like
      // Gecko) Chrome/58.0.3029.110 Safari/537.36")
      // .referrer("https://google.com")
      // .timeout(TIMEOUT).
      .get();

  private Function<Exception, Boolean> RETRY_CONDITION = (Exception e) -> e.getMessage().contains("Status=429");

  private Scraper(String initialUrl) throws IOException {
    this.scrapeUrl = initialUrl;
    loadDocument();
  }

  public static Scraper getInstance(String initialUrl) throws IOException {
    if (scraper == null) {
      scraper = new Scraper(initialUrl);
    }
    return scraper.updateScrapeUrl(initialUrl);
  }

  public static Scraper getInstance() {
    if (scraper == null) {
      throw new IllegalStateException("Scraper instance must be initialized with an initial URL.");
    }
    return scraper;
  }

  public Scraper updateScrapeUrl(String newUrl) throws IOException {
    this.scrapeUrl = newUrl;
    loadDocument();
    return scraper;
  }

  private void loadDocument() {
    log("Connecting to \"" + scrapeUrl + "\"", "INFO");
    try {
      doc = exponentialBackoff(CONNECT_TO_PAGE,
          RETRY_CONDITION);
    } catch (IOException e) {
      log("Failed to connect to \"" + scrapeUrl + "\": " + e.getMessage(), "ERROR");
    }

  }

  public Elements getElements(String selector) {
    if (doc == null)
      throw new IllegalStateException("Document must be loaded to select elements.");
    return doc.select(selector);
  }

  public Element getElement(String selector) {
    if (doc == null)
      throw new IllegalStateException("Document must be loaded to select elements.");
    return doc.selectFirst(selector);
  }

  private Document exponentialBackoff(Callable<Document> fn, Function<Exception, Boolean> condition)
      throws IOException {
    int retries = 0;

    while (retries < 15) {
      try {
        return fn.call();
      } catch (Exception e) {
        if (condition.apply(e)) {
          long wait = (long) Math.pow(2, retries) * 1000;
          log("Retrying after " + wait + " ms due to: " + e.getMessage(), "ERROR");

          try {
            Thread.sleep(wait);
          } catch (InterruptedException ie) {
            Thread.currentThread().interrupt();
            throw new IOException("Interrupted", ie);
          }
          retries++;
        } else {
          throw new IOException("Not retryable: " + e.getMessage());
        }
      }
    }

    throw new IOException("Retries exceeded");
  }

  public <T> T getContentOrFallback(Element el, T fallback, Class<T> T) {
    if (el != null) {
      T value = convertTextToType(el.text(), T);
      return (value != null) ? value : fallback;
    }
    return fallback;
  }

  public <T> T getContentOrFallback(String selector, T fallback, Class<T> T) {
    Element el = getElement(selector);
    if (el != null) {
      T value = convertTextToType(el.text(), T);
      return (value != null) ? value : fallback;
    }
    return fallback;
  }

  public <T> T convertTextToType(String text, Class<T> type) {
    try {
      if (type == Double.class) {
        boolean dotsAsThousandsSeparator = Pattern.compile("^(\\d{1,3})(\\.?\\d{3})*(,\\d+)?$").matcher(text).matches();

        if (dotsAsThousandsSeparator) {
          text = text.replace(".", "").replace(",", ".");
        } else {
          text = text.replace(",", "");
        }

        return type.cast(Double.valueOf(text));
      } else if (type == Integer.class) {
        // replace international thousands separators with "" to avoid errors
        // WARNING: this means it would work even when given a double, so be careful
        // when using it
        text = text.replace(".", "").replace(",", "");
        return type.cast(Integer.valueOf(text));
      } else if (type == String.class) {
        return type.cast(text);
      } else if (type == LocalDate.class) {
        return type.cast(LocalDate.parse(text, DATE_FORMATTER));
      } else if (type == LocalDateTime.class) {
        return type.cast(LocalDateTime.parse(text, DATE_TIME_FORMATTER));
      }
    } catch (NumberFormatException | DateTimeParseException e) {
      log("Invalid format: \"" + text + "\" for type " + type, "ERROR");
    }
    return null;
  }

  public String getScrapeUrl() {
    return scraper.scrapeUrl;
  }

  private void log(String err, String type) {
    try (BufferedWriter writer = new BufferedWriter(new FileWriter(LOG_FILE, true))) {
      writer.write("[" + type.toUpperCase() + "] @ " + LocalDateTime.now().toString());
      writer.newLine();
      writer.write(err);
      writer.newLine();

    } catch (IOException e) {
      System.err.println("Error writing log file: " + e.getMessage());
    }
  }

  public LocalDateTime getLocalDateTimeContent(String selector) {
    return getLocalDateTimeContent(selector, null);
  }

  public LocalDateTime getLocalDateTimeContent(Element el) {
    return getLocalDateTimeContent(el, null);
  }

  public LocalDateTime getLocalDateTimeContent(String selector, LocalDateTime fallback) {
    return getContentOrFallback(selector, fallback, LocalDateTime.class);
  }

  public LocalDateTime getLocalDateTimeContent(Element el, LocalDateTime fallback) {
    return getContentOrFallback(el, fallback, LocalDateTime.class);
  }

  public Double getDoubleContent(String selector) {
    return getContentOrFallback(selector, Double.NaN, Double.class);
  }

  public Double getDoubleContent(Element el) {
    return getContentOrFallback(el, Double.NaN, Double.class);
  }

  public Double getDoubleContent(String selector, Double fallback) {
    return getContentOrFallback(selector, fallback, Double.class);
  }

  public Double getDoubleContent(Element el, Double fallback) {
    return getContentOrFallback(el, fallback, Double.class);
  }

  public Integer getIntegerContent(String selector) {
    return getContentOrFallback(selector, null, Integer.class);
  }

  public Integer getIntegerContent(Element el) {
    return getContentOrFallback(el, null, Integer.class);
  }

  public Integer getIntegerContent(String selector, Integer fallback) {
    return getContentOrFallback(selector, fallback, Integer.class);
  }

  public Integer getIntegerContent(Element el, Integer fallback) {
    return getContentOrFallback(el, fallback, Integer.class);
  }

  public String getStringContent(String selector) {
    return getContentOrFallback(selector, null, String.class);
  }

  public String getStringContent(Element el) {
    return getContentOrFallback(el, null, String.class);
  }

  public String getStringContent(String selector, String fallback) {
    return getContentOrFallback(selector, fallback, String.class);
  }

  public String getStringContent(Element el, String fallback) {
    return getContentOrFallback(el, fallback, String.class);
  }

}
