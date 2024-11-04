package com.leo.scraper.test;

public class ExecuteWithDuration {
  private ExecuteWithDuration() {
  }

  // public static <T> T run(Callable<T> fn) throws Exception {
  // long startTime = System.currentTimeMillis();
  // T result = fn.call();
  // long endTime = System.currentTimeMillis();
  // long duration = endTime - startTime;
  // System.out.println("Execution time: " + duration + " ms");
  // return result;
  // }

  public static void run(Runnable fn) throws Exception {
    long startTime = System.currentTimeMillis();
    try {
      fn.run();
    } catch (Exception e) {
      System.out.println("Error occurred: " + e.getMessage());
      System.out.println("Execution time " + getDuration(startTime) + " ms");
    }

    System.out.println("Execution time: " + getDuration(startTime) + " ms");
  }

  private static long getDuration(long startTime) {
    long endTime = System.currentTimeMillis();
    return endTime - startTime;
  }
}
