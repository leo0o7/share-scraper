// package com.leo.scraper.share;
//
// import java.io.IOException;
// import java.time.LocalDate;
//
// import com.leo.scraper.Scraper;
// import com.leo.scraper.selectors.ShareInfoSelectors;
// import com.leo.scraper.url.BorsaItaliana;
//
// public class Share {
//
// private String URL;
// private String codiceIsin;
// private String codiceAlfanumerico;
// private String superSector;
// private String mercatoSegmento;
// private double maxOggi;
// private double maxAnno;
// private LocalDate maxAnnoDate;
// private double minOggi;
// private double minAnno;
// private LocalDate minAnnoDate;
// private double performance1Mese;
// private double performance6Mesi;
// private double performance1Anno;
//
// public Share(String codiceIsin) throws IOException {
// this.codiceIsin = codiceIsin;
// scrapeData();
// }
//
// private void scrapeData() throws IOException {
// URL = BorsaItaliana.getUrlDatiAzione(codiceIsin);
// Scraper scraper = Scraper.getInstance(URL);
//
// codiceAlfanumerico =
// scraper.getStringContent(ShareInfoSelectors.CODICE_ALFANUMERICO);
// superSector = scraper.getStringContent(ShareInfoSelectors.SUPER_SECTOR);
// mercatoSegmento =
// scraper.getStringContent(ShareInfoSelectors.MERCATO_SEGMENTO);
// maxOggi = scraper.getDoubleContent(ShareInfoSelectors.MAX_OGGI);
// minOggi = scraper.getDoubleContent(ShareInfoSelectors.MIN_OGGI);
// setAnnoFromProprietaryString(scraper.getStringContent(ShareInfoSelectors.MAX_ANNO),
// true);
// setAnnoFromProprietaryString(scraper.getStringContent(ShareInfoSelectors.MIN_ANNO),
// false);
// setPerformanceFromProprietaryString(scraper.getStringContent(ShareInfoSelectors.PF_1_MESE),
// "1M");
// setPerformanceFromProprietaryString(scraper.getStringContent(ShareInfoSelectors.PF_6_MESI),
// "6M");
// setPerformanceFromProprietaryString(scraper.getStringContent(ShareInfoSelectors.PF_1_ANNO),
// "1A");
// }
//
// public void setAnnoFromProprietaryString(String proprietaryString, boolean
// isMax) {
// if (proprietaryString == null || proprietaryString.isEmpty() ||
// proprietaryString.equals("")
// || proprietaryString.equals("-"))
// return;
//
// String[] arr = proprietaryString.split(" - ");
// double value = Double.valueOf(arr[0].replace(".", "").replace(",", "."));
// String[] dateArr = arr[1].split("/");
//
// LocalDate date = LocalDate.of(Integer.valueOf(dateArr[2]),
// Integer.valueOf(dateArr[1]),
// Integer.valueOf(dateArr[0]));
//
// if (isMax) {
// maxAnno = value;
// maxAnnoDate = date;
// } else {
// minAnno = value;
// minAnnoDate = date;
// }
// }
//
// public void setPerformanceFromProprietaryString(String proprietaryString,
// String range) {
// if (proprietaryString == null || proprietaryString.isEmpty() ||
// proprietaryString.equals(""))
// return;
//
// Double performance = Double.valueOf(proprietaryString.replace(".",
// "").replace(",", ".").replace("%", ""));
// switch (range.strip().toUpperCase()) {
// case "1M":
// performance1Mese = performance;
// break;
// case "6M":
// performance6Mesi = performance;
// break;
// case "1A":
// performance1Anno = performance;
// break;
//
// default:
// break;
// }
// }
//
// public String getCodiceIsin() {
// return codiceIsin;
// }
//
// public void setCodiceIsin(String codiceIsin) {
// this.codiceIsin = codiceIsin;
// }
//
// public String getCodiceAlfanumerico() {
// return codiceAlfanumerico;
// }
//
// public void setCodiceAlfanumerico(String codiceAlfanumerico) {
// this.codiceAlfanumerico = codiceAlfanumerico;
// }
//
// public String getSuperSector() {
// return superSector;
// }
//
// public void setSuperSector(String superSector) {
// this.superSector = superSector;
// }
//
// public String getMercatoSegmento() {
// return mercatoSegmento;
// }
//
// public void setMercatoSegmento(String mercatoSegmento) {
// this.mercatoSegmento = mercatoSegmento;
// }
//
// public double getMaxOggi() {
// return maxOggi;
// }
//
// public void setMaxOggi(double maxOggi) {
// this.maxOggi = maxOggi;
// }
//
// public double getMaxAnno() {
// return maxAnno;
// }
//
// public void setMaxAnno(double maxAnno) {
// this.maxAnno = maxAnno;
// }
//
// public LocalDate getMaxAnnoDate() {
// return maxAnnoDate;
// }
//
// public void setMaxAnnoDate(LocalDate maxAnnoDate) {
// this.maxAnnoDate = maxAnnoDate;
// }
//
// public double getMinOggi() {
// return minOggi;
// }
//
// public void setMinOggi(double minOggi) {
// this.minOggi = minOggi;
// }
//
// public double getMinAnno() {
// return minAnno;
// }
//
// public void setMinAnno(double minAnno) {
// this.minAnno = minAnno;
// }
//
// public LocalDate getMinAnnoDate() {
// return minAnnoDate;
// }
//
// public void setMinAnnoDate(LocalDate minAnnoDate) {
// this.minAnnoDate = minAnnoDate;
// }
//
// public double getPerformance1Mese() {
// return performance1Mese;
// }
//
// public void setPerformance1Mese(double performance1Mese) {
// this.performance1Mese = performance1Mese;
// }
//
// public double getPerformance6Mesi() {
// return performance6Mesi;
// }
//
// public void setPerformance6Mesi(double performance6Mesi) {
// this.performance6Mesi = performance6Mesi;
// }
//
// public double getPerformance1Anno() {
// return performance1Anno;
// }
//
// public void setPerformance1Anno(double performance1Anno) {
// this.performance1Anno = performance1Anno;
// }
//
// @Override
// public String toString() {
// return "Share {\n" +
// " codiceIsin: " + codiceIsin + "\n" +
// " codiceAlfanumerico: " + codiceAlfanumerico + "\n" +
// " superSector: " + superSector + "\n" +
// " mercatoSegmento: " + mercatoSegmento + "\n" +
// " maxOggi: " + maxOggi + "\n" +
// " maxAnno: " + maxAnno + "\n" +
// " maxAnnoDate: " + maxAnnoDate + "\n" +
// " minOggi: " + minOggi + "\n" +
// " minAnno: " + minAnno + "\n" +
// " minAnnoDate: " + minAnnoDate + "\n" +
// " performance1Mese: " + performance1Mese + "\n" +
// " performance6Mesi: " + performance6Mesi + "\n" +
// " performance1Anno: " + performance1Anno + "\n" +
// "}";
// }
//
// }
