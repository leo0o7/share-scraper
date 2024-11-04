package com.leo.scraper.selectors;

// not used anymore, see ShareProps and ShareParser
public class ShareInfoSelectors {
  private ShareInfoSelectors() {
  }

  public static final String CODICE_ISIN = Selectors.getComposedSelector(Selectors.Side.LEFT, 1);
  public static final String CODICE_ALFANUMERICO = Selectors.getComposedSelector(Selectors.Side.LEFT, 3);
  public static final String SUPER_SECTOR = Selectors.getComposedSelector(Selectors.Side.LEFT, 4);
  public static final String MERCATO_SEGMENTO = Selectors.getComposedSelector(Selectors.Side.LEFT, 5);
  public static final String MAX_OGGI = Selectors.getComposedSelector(Selectors.Side.RIGHT, 2);
  public static final String MIN_OGGI = Selectors.getComposedSelector(Selectors.Side.RIGHT, 4);
  public static final String MAX_ANNO = Selectors.getComposedSelector(Selectors.Side.RIGHT, 3);
  public static final String MIN_ANNO = Selectors.getComposedSelector(Selectors.Side.RIGHT, 5);
  public static final String PF_1_MESE = Selectors.getComposedSelector(Selectors.Side.RIGHT, 10);
  public static final String PF_6_MESI = Selectors.getComposedSelector(Selectors.Side.RIGHT, 11);
  public static final String PF_1_ANNO = Selectors.getComposedSelector(Selectors.Side.RIGHT, 12);
}
