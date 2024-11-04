package com.leo.scraper.test;

import com.leo.scraper.isin.IsinScraper;

import java.io.File;
import java.io.IOException;

public class TestIsinsScraper {
  private static File OUTPUT_FILE = new File("isins.txt");

  public static void run() {
    System.out.println("Using default output file: " + OUTPUT_FILE.getName());

    try {
      ExecuteWithDuration.run(() -> {
        try {
          IsinScraper scraper = new IsinScraper();
          scraper.fetchIsins(true);
          printResults();
        } catch (IOException e) {
        }
      });
    } catch (Exception e) {
      System.out.println("Error occurred: " + e.getMessage());
      e.printStackTrace();
    }
  }

  private static void printResults() {
    System.out.println("Output file: " + OUTPUT_FILE.getName());
    System.out.println("Output file size: " + OUTPUT_FILE.length() + " bytes");
  }

}
