package com.leo.scraper.share;

import java.io.IOException;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.util.HashMap;

import com.leo.scraper.share.parser.ShareParser;

public class Share {
  private HashMap<String, Object> properties = new HashMap<>();

  public Share(String codiceIsin) throws IOException {
    setCodiceIsin(codiceIsin);
    ShareParser.scrapeISIN(this);
  }

  public Object getProperty(String key) {
    return properties.get(key);
  }

  public void setProperty(String key, Object value) {
    properties.put(key, value);
  }

  @Override
  public String toString() {
    StringBuilder sb = new StringBuilder("Share {\n");

    // iterate from left_1 to left_16
    for (int i = 1; i <= 16; i++) {
      String leftKey = "left_" + i;
      String propName = ShareProps.rowToProp.get(leftKey);

      if (propName != null) {
        for (String prop : propName.split(",")) {
          Object value = properties.get(prop);
          sb.append("  ").append(prop).append(": ")
              .append(value != null ? value.toString() : "null")
              .append(",\n");
        }
      }
    }

    // iterate from right_1 to right_12
    for (int i = 1; i <= 12; i++) {
      String rightKey = "right_" + i;
      String propName = ShareProps.rowToProp.get(rightKey);

      if (propName != null) {
        for (String prop : propName.split(",")) {
          Object value = properties.get(prop);
          sb.append("  ").append(prop).append(": ")
              .append(value != null ? value.toString() : "null")
              .append(",\n");
        }
      }
    }

    // "Share {\n".length() = 8
    // if properties were added
    if (sb.length() > 8) {
      // remove trailing comma and \n
      sb.setLength(sb.length() - 2);
    }

    sb.append("\n}");
    return sb.toString();
  }

  // left Table Properties
  public String getCodiceIsin() {
    return (String) getProperty("codiceIsin");
  }

  public void setCodiceIsin(String codiceIsin) {
    setProperty("codiceIsin", codiceIsin);
  }

  public Double getIdStrumento() {
    return (Double) getProperty("idStrumento");
  }

  public void setIdStrumento(Double idStrumento) {
    setProperty("idStrumento", idStrumento);
  }

  public String getCodiceAlfanumerico() {
    return (String) getProperty("codiceAlfanumerico");
  }

  public void setCodiceAlfanumerico(String codiceAlfanumerico) {
    setProperty("codiceAlfanumerico", codiceAlfanumerico);
  }

  public String getSuperSector() {
    return (String) getProperty("superSector");
  }

  public void setSuperSector(String superSector) {
    setProperty("superSector", superSector);
  }

  public String getMercatoSegmento() {
    return (String) getProperty("mercatoSegmento");
  }

  public void setMercatoSegmento(String mercatoSegmento) {
    setProperty("mercatoSegmento", mercatoSegmento);
  }

  public Double getCapitalizzazioneDiMercato() {
    return (Double) getProperty("capitalizzazioneDiMercato");
  }

  public void setCapitalizzazioneDiMercato(Double capitalizzazioneDiMercato) {
    setProperty("capitalizzazioneDiMercato", capitalizzazioneDiMercato);
  }

  public Double getLottoMinimo() {
    return (Double) getProperty("lottoMinimo");
  }

  public void setLottoMinimo(Double lottoMinimo) {
    setProperty("lottoMinimo", lottoMinimo);
  }

  public String getFaseDiMercato() {
    return (String) getProperty("faseDiMercato");
  }

  public void setFaseDiMercato(String faseDiMercato) {
    setProperty("faseDiMercato", faseDiMercato);
  }

  public Double getPrezzoUltimoContratto() {
    return (Double) getProperty("prezzoUltimoContratto");
  }

  public void setPrezzoUltimoContratto(Double prezzoUltimoContratto) {
    setProperty("prezzoUltimoContratto", prezzoUltimoContratto);
  }

  public Double getVarPercentuale() {
    return (Double) getProperty("varPercentuale");
  }

  public void setVarPercentuale(Double varPercentuale) {
    setProperty("varPercentuale", varPercentuale);
  }

  public Double getVarAssoluta() {
    return (Double) getProperty("varAssoluta");
  }

  public void setVarAssoluta(Double varAssoluta) {
    setProperty("varAssoluta", varAssoluta);
  }

  public Double getPrMedioProgr() {
    return (Double) getProperty("prMedioProgr");
  }

  public void setPrMedioProgr(Double prMedioProgr) {
    setProperty("prMedioProgr", prMedioProgr);
  }

  public LocalDateTime getDataOraUltimoContratto() {
    return (LocalDateTime) getProperty("dataOraUltimoContratto");
  }

  public void setDataOraUltimoContratto(LocalDateTime dataOraUltimoContratto) {
    setProperty("dataOraUltimoContratto", dataOraUltimoContratto);
  }

  public Double getQuantitaUltimo() {
    return (Double) getProperty("quantitaUltimo");
  }

  public void setQuantitaUltimo(Double quantitaUltimo) {
    setProperty("quantitaUltimo", quantitaUltimo);
  }

  public Double getQuantitaTotale() {
    return (Double) getProperty("quantitaTotale");
  }

  public void setQuantitaTotale(Double quantitaTotale) {
    setProperty("quantitaTotale", quantitaTotale);
  }

  public Integer getNumeroContratti() {
    return (Integer) getProperty("numeroContratti");
  }

  public void setNumeroContratti(Integer numeroContratti) {
    setProperty("numeroContratti", numeroContratti);
  }

  // right Table Properties
  public Double getControvalore() {
    return (Double) getProperty("controvalore");
  }

  public void setControvalore(Double controvalore) {
    setProperty("controvalore", controvalore);
  }

  public Double getMaxOggi() {
    return (Double) getProperty("maxOggi");
  }

  public void setMaxOggi(Double maxOggi) {
    setProperty("maxOggi", maxOggi);
  }

  public Double getMaxAnno() {
    return (Double) getProperty("maxAnno");
  }

  public void setMaxAnno(Double maxAnno) {
    setProperty("maxAnno", maxAnno);
  }

  public LocalDate getMaxAnnoDate() {
    return (LocalDate) getProperty("maxAnnoDate");
  }

  public void setMaxAnnoDate(LocalDate maxAnnoDate) {
    setProperty("maxAnnoDate", maxAnnoDate);
  }

  public Double getMinOggi() {
    return (Double) getProperty("minOggi");
  }

  public void setMinOggi(Double minOggi) {
    setProperty("minOggi", minOggi);
  }

  public Double getMinAnno() {
    return (Double) getProperty("minAnno");
  }

  public void setMinAnno(Double minAnno) {
    setProperty("minAnno", minAnno);
  }

  public LocalDate getMinAnnoDate() {
    return (LocalDate) getProperty("minAnnoDate");
  }

  public void setMinAnnoDate(LocalDate minAnnoDate) {
    setProperty("minAnnoDate", minAnnoDate);
  }

  public Double getChiusuraPrecedente() {
    return (Double) getProperty("chiusuraPrecedente");
  }

  public void setChiusuraPrecedente(Double chiusuraPrecedente) {
    setProperty("chiusuraPrecedente", chiusuraPrecedente);
  }

  public Double getPrezzoRiferimento() {
    return (Double) getProperty("prezzoRiferimento");
  }

  public void setPrezzoRiferimento(Double prezzoRiferimento) {
    setProperty("prezzoRiferimento", prezzoRiferimento);
  }

  public LocalDateTime getDataOraPrezzoRifermento() {
    return (LocalDateTime) getProperty("dataOraPrezzoRifermento");
  }

  public void setDataOraPrezzoRifermento(LocalDateTime dataOraPrezzoRifermento) {
    setProperty("dataOraPrezzoRifermento", dataOraPrezzoRifermento);
  }

  public Double getPrezzoUfficiale() {
    return (Double) getProperty("prezzoUfficiale");
  }

  public void setPrezzoUfficiale(Double prezzoUfficiale) {
    setProperty("prezzoUfficiale", prezzoUfficiale);
  }

  public LocalDate getDataPrezzoUfficiale() {
    return (LocalDate) getProperty("dataPrezzoUfficiale");
  }

  public void setDataPrezzoUfficiale(LocalDate dataOraPrezzoUfficiale) {
    setProperty("dataPrezzoUfficiale", dataOraPrezzoUfficiale);
  }

  public Double getAperturaOdierna() {
    return (Double) getProperty("aperturaOdierna");
  }

  public void setAperturaOdierna(Double aperturaOdierna) {
    setProperty("aperturaOdierna", aperturaOdierna);
  }

  public Double getPerformance1Mese() {
    return (Double) getProperty("performance1Mese");
  }

  public void setPerformance1Mese(Double performance1Mese) {
    setProperty("performance1Mese", performance1Mese);
  }

  public Double getPerformance6Mesi() {
    return (Double) getProperty("performance6Mesi");
  }

  public void setPerformance6Mesi(Double performance6Mesi) {
    setProperty("performance6Mesi", performance6Mesi);
  }

  public Double getPerformance1Anno() {
    return (Double) getProperty("performance1Anno");
  }

  public void setPerformance1Anno(Double performance1Anno) {
    setProperty("performance1Anno", performance1Anno);
  }
}
