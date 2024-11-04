# Share Scraper

A Java-based scraper for fetching financial data using ISIN codes. This project provides classes to retrieve, parse, and store share details, designed to handle common issues like rate limiting and data parsing.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Requirements](#requirements)
- [Setup](#setup)
- [Usage](#usage)
- [Error Handling](#error-handling)

## Overview

The Share Scraper project scrapes financial data from the Borsa Italiana website using ISIN codes. It includes functionalities for:

- Retrieving and storing ISIN data
- Fetching share details
- Handling rate limits and retry logic

## Features

- **Exponential Backoff**: Automatically retries requests with exponential backoff on rate limit errors (`429 Too Many Requests`).
- **Type-Safe Data Parsing**: Ensures that scraped data is parsed and stored in the correct format.
- **Logging**: Logs all errors and relevant info to `scraper.log`.

## Requirements

- **Maven**: Dependency management and build automation
- **Internet Connection**: Required to fetch data from external sources.

## Setup

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/yourusername/share-scraper.git
   cd share-scraper
   ```

2. **Compile the Project**:

   ```bash
   mvn clean install
   ```

3. **Run the Application**:

   ```bash
   mvn exec:java -Dexec.mainClass=com.leo.scraper.Main
   ```

## Usage

Run the `Main` class to start the application. It provides a simple command-line interface (CLI) for testing various scraping functionalities.

1. Run the program, and select the desired option:

   - **Option 1**: Scrapes share data using ISINs.
   - **Option 2**: Scrapes a list of ISINs.
   - **Option 3**: Loads ISINs from a local file.

2. **Sample Command-Line Interaction**:

   ```plaintext
   ============================
   Welcome to the share scraper

   What do you want to do?
    1. Test scraping shares (requires isins.txt file)
    2. Test scraping ISINs
    3. Test loading ISINs
   Enter your choice: 1

   Selected: "1. Test scraping shares"
   Output file (default: shares_output.txt): [Press Enter for default or specify a path]
   ```

## Error Handling

Errors and log entries are written to `scraper.log`:

- **INFO**: Logs general information, such as connection attempts.
- **ERROR**: Logs errors, such as retries or format issues.
