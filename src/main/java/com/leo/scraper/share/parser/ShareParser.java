package com.leo.scraper.share.parser;

import java.io.IOException;
import java.lang.reflect.Method;

import org.jsoup.nodes.Element;
import org.jsoup.select.Elements;

import com.leo.scraper.Scraper;
import com.leo.scraper.selectors.Selectors;
import com.leo.scraper.share.Share;
import com.leo.scraper.share.ShareProps;
import com.leo.scraper.url.BorsaItaliana;

public class ShareParser {

  private ShareParser() {
  }

  public static void scrapeISIN(Share s) throws IOException {
    String URL = BorsaItaliana.getUrlDatiAzione((String) s.getProperty("codiceIsin"));
    Scraper scraper = Scraper.getInstance(URL);

    Elements left = scraper
        .getElements(Selectors.getLeftOrRightTableSelector(Selectors.Side.LEFT) + " span.t-text.-right");

    insertTableInfo(s, left, "left");

    Elements right = scraper
        .getElements(Selectors.getLeftOrRightTableSelector(Selectors.Side.RIGHT) + " span.t-text.-right");

    insertTableInfo(s, right, "right");
  }

  private static void insertTableInfo(Share s, Elements elements, String side) {
    int currRow = 1;
    for (Element element : elements) {
      if ((side.equals("left") && currRow > 16) || (side.equals("right") && currRow > 12))
        break;
      if (SpecialRows.isPriceDateReferenceRow(side, currRow)) {
        SpecialRows.insertPriceDateReference(s, element, currRow, side);
        currRow++;
        continue;
      }
      if (SpecialRows.isPerformanceRow(side, currRow)) {
        SpecialRows.insertPerformanceInfo(s, element, currRow);
        currRow++;
        continue;
      }
      insertRowInfo(s, side + "_" + currRow, element);
      currRow++;
    }
  }

  private static void insertRowInfo(Share s, String key, Element element) {
    String propName = ShareProps.rowToProp.get(key);
    Class<?> type = ShareProps.propToType.get(propName);

    s.setProperty(propName, getRowInfo(element, type));
  }

  private static Object getRowInfo(Element element, Class<?> type) {
    Scraper scraper = Scraper.getInstance();
    String methodName = "get" + type.getSimpleName() + "Content";

    try {
      Method method = scraper.getClass().getMethod(methodName, Element.class);

      return method.invoke(scraper, element);
    } catch (NoSuchMethodException e) {
      System.err.println("No such method: " + methodName);
    } catch (Exception e) {
      System.err.println("Error invoking method (" + methodName + ") : " + e.getMessage());
    }

    return null;
  }

}
