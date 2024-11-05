package com.leo.scraper.share.parser;

import com.leo.scraper.share.Share;
import org.jsoup.nodes.Element;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.time.LocalDate;
import java.time.LocalDateTime;

import static org.junit.jupiter.api.Assertions.*;

public class SpecialRowsTest {
  private static Share share;

  @BeforeAll
  public static void setUp() throws IOException {
    share = new Share("");
  }

  @Test
  public void testIsPriceDateReferenceRow() {
    assertTrue(SpecialRows.isPriceDateReferenceRow(SpecialRows.YEAR_INFO_SIDE,
        3));
    assertTrue(SpecialRows.isPriceDateReferenceRow(SpecialRows.REFERENCE_PRICE_SIDE,
        7));
    assertTrue(SpecialRows.isPriceDateReferenceRow(SpecialRows.UFFICIAL_PRICE_SIDE,
        8));
    assertFalse(SpecialRows.isPriceDateReferenceRow(SpecialRows.PERFORMANCE_INFO_SIDE,
        10));
  }

  @Test
  public void testInsertPriceDateReferenceWithValidData() {
    String proprietaryString = "1234,56 - 04/11/24 17.45.00";
    Element mockElement = new Element("div").text(proprietaryString);

    SpecialRows.insertPriceDateReference(share, mockElement, 7,
        SpecialRows.REFERENCE_PRICE_SIDE);

    assertEquals(1234.56, share.getProperty("prezzoRiferimento"));
    assertEquals(LocalDateTime.of(2024, 11, 4, 17, 45),
        share.getProperty("dataOraPrezzoRifermento"));
  }

  @Test
  public void testInsertPriceDateReferenceWithOnlyDate() {
    String proprietaryString = "1234,56 - 04/11/24";
    Element mockElement = new Element("div").text(proprietaryString);
    SpecialRows.insertPriceDateReference(share, mockElement, 8,
        SpecialRows.REFERENCE_PRICE_SIDE);

    assertEquals(1234.56, share.getProperty("prezzoUfficiale"));
    assertEquals(LocalDate.of(2024, 11, 4), share.getProperty("dataPrezzoUfficiale"));
  }

  @Test
  public void testInsertPriceDateReferenceWithInvalidData() {
    String proprietaryString = "-";
    Element mockElement = new Element("div").text(proprietaryString);
    SpecialRows.insertPriceDateReference(share, mockElement, 7,
        SpecialRows.REFERENCE_PRICE_SIDE);

    assertNull(share.getProperty("prezzoRiferimento"), "Expected null due to invalid proprietary string");
    assertNull(share.getProperty("dataOraPrezzoRiferimento"), "Expected null due to invalid proprietary string");
  }

  @Test
  public void testInsertPerformanceInfoWithValidData() {
    String proprietaryString = "10.5%";
    Element mockElement = new Element("div").text(proprietaryString);
    SpecialRows.insertPerformanceInfo(share, mockElement, 10);

    assertEquals(10.5, share.getProperty("performance1Mese"));
  }

  @Test
  public void testInsertPerformanceInfoWithEmptyData() {
    String proprietaryString = "";
    Element mockElement = new Element("div").text(proprietaryString);
    SpecialRows.insertPerformanceInfo(share, mockElement, 10);

    assertNull(share.getProperty("performance1Mese"), "Expected null due to empty proprietary string");
  }

  @Test
  public void testInsertPerformanceInfoWithInvalidData() {
    String proprietaryString = "not a number";
    Element mockElement = new Element("div").text(proprietaryString);
    SpecialRows.insertPerformanceInfo(share, mockElement, 10);

    assertNull(share.getProperty("performance1Mese"), "Expected null due to invalid performance string");
  }
}
