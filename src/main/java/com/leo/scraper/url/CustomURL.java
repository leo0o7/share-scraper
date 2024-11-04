package com.leo.scraper.url;

import java.net.MalformedURLException;
import java.net.URLStreamHandler;
import java.net.URL;

public class CustomURL {
  private URL url;

  public CustomURL(String spec) throws MalformedURLException {
    this.url = new URL(spec);
  }

  public CustomURL(URL context, String spec) throws MalformedURLException {
    this.url = new URL(context, spec);
  }

  public CustomURL(URL context, String spec, URLStreamHandler handler) throws MalformedURLException {
    this.url = new URL(context, spec, handler);
  }

  public CustomURL(String protocol, String host, int port, String file) throws MalformedURLException {
    this.url = new URL(protocol, host, port, file);
  }

  public CustomURL(String protocol, String host, String file) throws MalformedURLException {
    this.url = new URL(protocol, host, file);
  }

  public void addParameter(String name, String value) throws MalformedURLException {
    String hasQuery = url.getQuery();
    char delimiter = hasQuery == null ? '?' : '&';

    url = new URL(url.toString() + delimiter + name + '=' + value);
  }

  public URL getUrl() {
    return url;
  }

  @Override
  public String toString() {
    return url.toString();
  }
}
