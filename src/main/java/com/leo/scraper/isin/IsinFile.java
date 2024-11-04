package com.leo.scraper.isin;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;

public class IsinFile {
  private static String FILE_PATH = "isins.txt";
  private static IsinFile instance;
  private File file;

  public static IsinFile getInstance() throws IOException {
    if (instance == null) {
      instance = new IsinFile();
    }
    return instance;
  }

  private IsinFile() throws IOException {
    file = new File(FILE_PATH);
    createFile();
  }

  private void createFile() throws IOException {
    if (!file.exists()) {
      file.createNewFile();
    }
  }

  public void addNewLine(String line) throws IOException {
    FileWriter fileWriter = new FileWriter(FILE_PATH, true);
    fileWriter.write(line + System.lineSeparator());
    fileWriter.close();
  }

  public FileReader readFile() throws FileNotFoundException {
    return new FileReader(FILE_PATH);
  }

}
