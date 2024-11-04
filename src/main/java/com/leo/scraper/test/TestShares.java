package com.leo.scraper.test;

import com.leo.scraper.isin.IsinStore;
import com.leo.scraper.share.Share;

import java.io.BufferedWriter;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;

// TEST "TELECOM "AND "POSTE" ISINS
//Share share1 = new Share("IT0003796171");
//Share share2 = new Share("IT0003497168");
//System.out.println("============");
//System.out.println(share1);
//System.out.println("============");
//System.out.println(share2);

public class TestShares {
  private static File OUTPUT_FILE = new File("shares_output.txt");

  public static void run() {
    run("");
  }

  public static void run(String output_file) {
    if (!output_file.trim().isEmpty())
      OUTPUT_FILE = new File(output_file.trim());
    else
      System.out.println("Using default output file: " + OUTPUT_FILE.getName());

    try {
      ExecuteWithDuration.run(() -> {
        try {
          IsinStore store = IsinStore.getInstance();
          int storeSize = store.getShareToIsin().size();
          scrapeShares(store, storeSize);
        } catch (Exception e) {
        }
      });
    } catch (Exception e) {
      System.out.println("Error occurred: " + e.getMessage());
      e.printStackTrace();
    }
  }

  private static void printResults(int storeSize, int isinCount) {
    System.out.println("\n============");
    System.out.println("ISINs in store: " + storeSize);
    System.out.println("Total ISINs scraped: " + isinCount);
    System.out.println("Output file: " + OUTPUT_FILE.getName());
    System.out.println("Output file size: " + OUTPUT_FILE.length() + " bytes");
  }

  private static int scrapeShares(IsinStore store, int storeSize) {
    int isinCount = 0;
    try (BufferedWriter writer = new BufferedWriter(new FileWriter(OUTPUT_FILE))) {

      for (String isin : store.getShareToIsin().values()) {
        System.out.print("Scraping " + isin.strip() + "...");
        Share s = new Share(isin);

        writer.write(s.toString());
        writer.newLine();

        isinCount++;

        System.out.print("Done! (" + isinCount + "/" + storeSize + ")\r");
        System.out.flush();
      }

    } catch (IOException e) {
      System.err.println("Error writing to file: " + e.getMessage());
    }

    printResults(storeSize, isinCount);
    return isinCount;
  }

}
