package com.leo.scraper;

import static org.junit.jupiter.api.Assertions.*;
import org.junit.jupiter.api.*;

import org.jsoup.nodes.Element;
import org.jsoup.nodes.TextNode;
import org.jsoup.select.Elements;

public class ScraperElementsTest extends BaseScraperTest {
  @Test
  public void testGetElements() {
    Elements elements = scraper.getElements("body");
    assertNotNull(elements);
  }

  @Test
  public void testGetElement() {
    Element element = scraper.getElement("head");
    assertNotNull(element);
  }

  @Test
  public void testElementContentOrFallback() {
    Element mockElement = new Element("div");
    mockElement.appendChild(new TextNode("Expected content"));

    String content = scraper.getContentOrFallback(mockElement, "Fallback content", String.class);
    assertEquals("Expected content", content);

    content = scraper.getContentOrFallback(".foo", "Fallback content", String.class);
    assertEquals("Fallback content", content);
  }

  @Test
  public void testGetIntegerContent() {
    Element element = new Element("div").text("12345");
    Integer result = scraper.getIntegerContent(element, 0);
    assertEquals(12345, result);

    element = new Element("div").text("");
    Integer fallback = scraper.getIntegerContent(element, 0);
    assertEquals(0, fallback, "Getting empty content should return fallback value");
  }

  @Test
  public void testGetDoubleContent() {
    Element element = new Element("div").text("123.45");
    Double result = scraper.getDoubleContent(element, 0.0);
    assertEquals(123.45, result, "Getting content should return value");

    element = new Element("div").text("");
    Double fallback = scraper.getDoubleContent(element, 0.0);
    assertEquals(0.0, fallback, "Getting empty content should return fallback value");
  }

}
