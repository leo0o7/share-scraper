package com.leo.scraper.isin;

import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;

public class IsinLoader {
  public static void load(IsinStore store) throws FileNotFoundException, IOException {
    String[] lines = getIsinFileLines();

    for (String line : lines) {
      String[] arr = line.split(" & ");

      String isin = arr[0].strip();
      String shareName = arr[1].strip();

      store.getIsinToShare().put(isin, shareName);
      store.getShareToIsin().put(shareName, isin);
    }
  }

  private static String[] getIsinFileLines() throws FileNotFoundException, IOException {
    FileReader r = IsinFile.getInstance().readFile();

    StringBuffer sb = new StringBuffer();
    int i;
    while ((i = r.read()) != -1) {
      sb.append((char) i);
    }
    return sb.toString().split(System.lineSeparator());
  }

}
