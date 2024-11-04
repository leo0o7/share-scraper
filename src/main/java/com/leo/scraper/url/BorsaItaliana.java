package com.leo.scraper.url;

import java.net.MalformedURLException;

public class BorsaItaliana {
  private BorsaItaliana() {
  }

  private static final String DEFAULT_LANG = "it";
  private static final String DEFAULT_PROTOCOL = "https";
  private static final String HOST = "www.borsaitaliana.it";
  private static final String SHARES_PATH = "/borsa/azioni/";
  private static final String FULL_DATA_PATH = "dati-completi.html";
  private static final String ACTIONS_LIST_PATH = "listino-a-z.html";

  public static String getBaseUrl() throws MalformedURLException {
    CustomURL url = createURL(DEFAULT_PROTOCOL, HOST, SHARES_PATH);
    return url.toString();
  }

  public static String getUrlDatiAzione(String isin) throws MalformedURLException {
    CustomURL url = createURL(DEFAULT_PROTOCOL, HOST, SHARES_PATH + FULL_DATA_PATH);
    url.addParameter("isin", isin);
    url.addParameter("lang", DEFAULT_LANG);
    return url.toString();
  }

  public static String getUrlListinoAzioni(char letter, int page) throws MalformedURLException {
    CustomURL url = createURL(DEFAULT_PROTOCOL, HOST, SHARES_PATH + ACTIONS_LIST_PATH);
    url.addParameter("initial", String.valueOf(letter));
    url.addParameter("lang", DEFAULT_LANG);
    url.addParameter("page", String.valueOf(page));
    return url.toString();
  }

  public static String getUrlListinoAzioni() throws MalformedURLException {
    CustomURL url = createURL(DEFAULT_PROTOCOL, HOST, SHARES_PATH + ACTIONS_LIST_PATH);
    url.addParameter("lang", DEFAULT_LANG);
    return url.toString();
  }

  private static CustomURL createURL(String protocol, String domain, String path) throws MalformedURLException {
    return new CustomURL(protocol, domain, path);
  }

}
