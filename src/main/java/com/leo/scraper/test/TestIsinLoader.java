package com.leo.scraper.test;

import com.leo.scraper.isin.IsinStore;

import java.io.File;
import java.io.IOException;

public class TestIsinLoader {
  private static File INPUT_FILE = new File("isins.txt");

  public static void run() {
    System.out.println("Using default input file: " + INPUT_FILE.getName());

    try {
      ExecuteWithDuration.run(() -> {
        try {
          printResults(IsinStore.getInstance());
        } catch (IOException e) {
        }
      });
    } catch (Exception e) {
      System.out.println("Error occurred: " + e.getMessage());
      e.printStackTrace();
    }
  }

  private static void printResults(IsinStore store) {
    System.out.println("============");
    System.out.println("Input file: " + INPUT_FILE.getName());
    System.out.println("Input file size: " + INPUT_FILE.length() + " bytes");
    System.out.println("Loaded ISIN to share name mappings: " + store.getIsinToShare().size());
    System.out.println("Loaded share name to ISIN mappings: " + store.getShareToIsin().size());
  }

}
