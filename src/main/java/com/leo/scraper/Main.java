package com.leo.scraper;

import java.util.Scanner;

import com.leo.scraper.test.TestIsinLoader;
import com.leo.scraper.test.TestIsinsScraper;
import com.leo.scraper.test.TestShares;

public class Main {

  public static void main(String[] args) {
    Scanner s = new Scanner(System.in);

    System.out.println("=".repeat(26));
    System.out.println("Welcome to the share scraper");
    System.out.println("\n");
    System.out.println("What do you want to do?");
    System.out.println(" 1. Test scraping shares (requires isins.txt files)");
    System.out.println(" 2. Test scraping ISINs");
    System.out.println(" 3. Test loading ISINs");
    System.out.print("Enter your choice: ");

    int selected = s.nextInt();
    s.nextLine();

    System.out.println("\n");

    switch (selected) {
      case 1:
        System.out.println("Selected:  \"1. Test scraping shares\"");
        System.out.print("Output file (default: shares_output.txt): ");
        String output = s.nextLine().trim();
        System.out.println("=".repeat(26));
        System.out.println("\n");
        TestShares.run(output);
        break;
      case 2:
        System.out.println("Selected:  \"2. Test scraping ISINs\"");
        System.out.println("=".repeat(26));
        System.out.println("\n");
        TestIsinsScraper.run();
        break;
      case 3:
        System.out.println("Selected:  \"3. Test loading ISINs\"");
        System.out.println("=".repeat(26));
        System.out.println("\n");
        TestIsinLoader.run();
        break;
      default:
        s.close();
        throw new IllegalArgumentException("Invalid option");
    }
    s.close();
  }

}
