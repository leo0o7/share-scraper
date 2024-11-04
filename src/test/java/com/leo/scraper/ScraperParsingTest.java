
package com.leo.scraper;

import static org.junit.jupiter.api.Assertions.*;

import java.time.LocalDate;
import java.time.LocalDateTime;

import org.junit.jupiter.api.*;

import org.jsoup.nodes.Element;

public class ScraperParsingTest extends BaseScraperTest {
  @Test
  public void testDoubleParsingValidFormat() {
    String validDouble = "1.234,56";
    Double result = scraper.convertTextToType(validDouble, Double.class);
    assertEquals(1234.56, result, "Double string should be parsed");
  }

  @Test
  public void testDoubleParsingInvalidFormat() {
    String invalidDouble = "1.234 56";
    Double result = scraper.convertTextToType(invalidDouble, Double.class);
    assertNull(result, "Invalid double strings should resolve to null");
  }

  @Test
  public void testIntegerParsing() {
    String validInteger = "1234";
    Integer result = scraper.convertTextToType(validInteger, Integer.class);
    assertEquals(1234, result, "Integer strings should be parsed");
  }

  @Test
  public void testInvalidIntegerParsing() {
    String validInteger = "1234 56";
    Integer result = scraper.convertTextToType(validInteger, Integer.class);
    assertNull(result, "Invalid integer strings should resolve to null");
  }

  @Test
  public void testValidDateParsing() {
    String validDate = "20/04/24";
    LocalDate expected = LocalDate.of(2024, 4, 20);
    LocalDate result = scraper.convertTextToType(validDate, LocalDate.class);
    assertEquals(expected, result, "Valid date strings should be parsed");
  }

  @Test
  public void testInvalidDateFormatHandling() {
    String invalidDate = "invalid-date-format";
    LocalDateTime result = scraper.convertTextToType(invalidDate, LocalDateTime.class);
    assertNull(result, "Expected null due to date parsing failure");
  }

  @Test
  public void testDateTimeContentParsing() {
    Element element = new Element("div").text("31/12/20 - 23.59.59");
    LocalDateTime result = scraper.getContentOrFallback(element, null, LocalDateTime.class);
    assertNotNull(result);
  }
}
