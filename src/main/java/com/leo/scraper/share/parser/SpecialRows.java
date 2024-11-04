package com.leo.scraper.share.parser;

import com.leo.scraper.share.Share;
import com.leo.scraper.share.ShareProps;
import com.leo.scraper.Scraper;

import java.time.LocalDate;
import java.time.LocalDateTime;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import org.jsoup.nodes.Element;

public class SpecialRows {

  public static final String YEAR_INFO_SIDE = "right";
  public static final List<Integer> YEAR_INFO_ROWS = List.of(3, 5);
  public static final String REFERENCE_PRICE_SIDE = "right";
  public static final int REFERENCE_PRICE_ROW = 7;
  public static final String UFFICIAL_PRICE_SIDE = "right";
  public static final int UFFICIAL_PRICE_ROW = 8;
  public static final String PERFORMANCE_INFO_SIDE = "right";
  public static final List<Integer> PERFORMANCE_INFO_ROWS = List.of(10, 11, 12);

  public static boolean isPriceDateReferenceRow(String side, int currRow) {
    return isYearRow(side, currRow) || isReferencePriceRow(side, currRow) || isUfficialPriceRow(side, currRow);
  }

  public static boolean isYearRow(String side, int currRow) {
    return side == YEAR_INFO_SIDE && YEAR_INFO_ROWS.contains(currRow);
  }

  public static boolean isReferencePriceRow(String side, int currRow) {
    return side == REFERENCE_PRICE_SIDE && REFERENCE_PRICE_ROW == (currRow);
  }

  public static boolean isUfficialPriceRow(String side, int currRow) {
    return side == UFFICIAL_PRICE_SIDE && UFFICIAL_PRICE_ROW == (currRow);
  }

  public static boolean isPerformanceRow(String side, int currRow) {
    return side == PERFORMANCE_INFO_SIDE && PERFORMANCE_INFO_ROWS.contains(currRow);
  }

  public static void insertPriceDateReference(Share s, Element el, int row, String side) {
    Scraper scraper = Scraper.getInstance();
    String proprietaryString = scraper.getStringContent(el);
    String[] props = ShareProps.rowToProp.get(side + "_" + row).split(",");
    String valueProp = props[0].strip();
    String dateProp = props[1].strip();

    if (proprietaryString == null || proprietaryString.isEmpty() || proprietaryString.equals("")
        || proprietaryString.equals("-"))
      return;

    String[] arr = proprietaryString.split(" - ");
    Double value = scraper.convertTextToType(arr[0], Double.class);
    s.setProperty(valueProp, value);

    // if no date found set it to null and return
    if (arr.length < 2) {
      s.setProperty(dateProp, null);
      return;
    }

    // the first group matches the date
    // (\d{2}\/\d{2}\/\d{2})
    // the second group matches the time
    // (:?\s\d{2}.\d{2}.\d{2})?
    // it's non capturing so it's not required for the match to happen
    // so both `04/11/24` and `04/11/24 17.45.00` will be captured
    Matcher localDateMatcher = Pattern.compile("(\\d{2}\\/\\d{2}\\/\\d{2})(:?\\s\\d{2}.\\d{2}.\\d{2})?")
        .matcher(arr[1]);

    if (localDateMatcher.matches()) {
      // if it has two groups it's a date and time
      if (localDateMatcher.group(2) != null) {
        // replacements to match the Scraper standard formatter
        arr[1] = arr[1].replace(" ", " - ");
        LocalDateTime dateTime = scraper.convertTextToType(arr[1], LocalDateTime.class);
        s.setProperty(dateProp, dateTime);
      } else {
        LocalDate date = scraper.convertTextToType(arr[1], LocalDate.class);
        s.setProperty(dateProp, date);
      }
    }

  }

  public static void insertPerformanceInfo(Share s, Element el, int row) {
    Scraper scraper = Scraper.getInstance();
    String proprietaryString = scraper.getStringContent(el);
    String propName = ShareProps.rowToProp.get(PERFORMANCE_INFO_SIDE + "_" + row);

    if (proprietaryString == null || proprietaryString.isEmpty() || proprietaryString.equals(""))
      return;

    String formattedString = proprietaryString.replace(".", "").replace(",", ".").replace("%", "");
    Double performance = scraper.convertTextToType(formattedString, Double.class);

    s.setProperty(propName, performance);
  }

}
