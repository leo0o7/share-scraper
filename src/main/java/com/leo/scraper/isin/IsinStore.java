package com.leo.scraper.isin;

import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.HashMap;

public class IsinStore {
  private static IsinStore instance;
  private HashMap<String, String> isinToShare = new HashMap<String, String>();
  private HashMap<String, String> shareToIsin = new HashMap<String, String>();

  public static IsinStore getInstance() throws FileNotFoundException, IOException {
    if (instance == null) {
      instance = new IsinStore();
    }
    return instance;
  }

  private IsinStore() throws FileNotFoundException, IOException {
    IsinLoader.load(this);
  }

  public String getIsinByShare(String share) {
    return shareToIsin.get(share);
  }

  public String getShareByIsin(String isin) {
    return isinToShare.get(isin);
  }

  public HashMap<String, String> getIsinToShare() {
    return isinToShare;
  }

  public void setIsinToShare(HashMap<String, String> isinToShare) {
    this.isinToShare = isinToShare;
  }

  public HashMap<String, String> getShareToIsin() {
    return shareToIsin;
  }

  public void setShareToIsin(HashMap<String, String> shareToIsin) {
    this.shareToIsin = shareToIsin;
  }

  @Override
  public String toString() {
    StringBuilder sb = new StringBuilder();
    sb.append("IsinStore Summary:\n");
    sb.append("-------------------------------------------------\n");
    sb.append("ISIN to Share Map (Size: ").append(isinToShare.size()).append(")\n");
    sb.append("-------------------------------------------------\n");

    isinToShare.forEach((isin, share) -> {
      sb.append(isin).append(" => ").append(share).append("\n");
    });

    sb.append("\nShare to ISIN Map (Size: ").append(shareToIsin.size()).append(")\n");
    sb.append("-------------------------------------------------\n");

    shareToIsin.forEach((share, isin) -> {
      sb.append(share).append(" => ").append(isin).append("\n");
    });

    return sb.toString();
  }

}
