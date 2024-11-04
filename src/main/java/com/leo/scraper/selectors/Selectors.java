package com.leo.scraper.selectors;

public class Selectors {
  private Selectors() {
  }

  public enum Side {
    LEFT, RIGHT
  }

  // String wrapperTableSelector = "div.l-box.-prl.l-screen.-sm-half.-md-half";
  public static String wrapperGridSelector = "article.l-grid__cell";
  public static String tableSelector = "table.m-table.-clear-m";
  public static String rightSideTextSelector = "span.t-text.-right";

  public static String getRowSelector(int rowNumber) {
    return "tr:nth-of-type(" + rowNumber + ")";
  }

  public static String getTableWrapperSelector(Side side) {
    if (side == Side.LEFT) {
      return "div.l-box.-prl.l-screen.-sm-half.-md-half:nth-of-type(1)";
    } else if (side == Side.RIGHT) {
      return "div.l-box.-prl.l-screen.-sm-half.-md-half:nth-of-type(2)";
    }
    return null;
  }

  public static String getLeftOrRightTableSelector(Side side) {
    return wrapperGridSelector + " " + getTableWrapperSelector(side) + " " + tableSelector + " tbody";
  }

  public static String getComposedSelector(Side side, int rowNumber) {
    return wrapperGridSelector + " " + getTableWrapperSelector(side) + " " + tableSelector + " "
        + getRowSelector(rowNumber) + " " + rightSideTextSelector;
  }

  public static String getIsinSelector() {
    // return "table.m-table.-firstlevel article.u-hidden.-sm.-md
    // div.l-box.-pb.l-scren.-xs-15
    // a[href*=\"/borsa/azioni/obbligazioni-convertibili/scheda/\"]";
    // return "a[href*=\"/borsa/azioni/scheda\"]";
    return "a.u-hidden.-xs";
  }
}
