package com.leo.scraper.share;

import java.time.LocalDate;
import java.time.LocalDateTime;
import java.util.HashMap;

public class ShareProps {

  public static final HashMap<String, String> rowToProp = new HashMap<String, String>() {
    {
      put("left_1", "codiceIsin");
      put("left_2", "idStrumento");
      put("left_3", "codiceAlfanumerico");
      put("left_4", "superSector");
      put("left_5", "mercatoSegmento");
      put("left_6", "capitalizzazioneDiMercato");
      put("left_7", "lottoMinimo");
      put("left_8", "faseDiMercato");
      put("left_9", "prezzoUltimoContratto");
      put("left_10", "varPercentuale");
      put("left_11", "varAssoluta");
      put("left_12", "prMedioProgr");
      put("left_13", "dataOraUltimoContratto");
      put("left_14", "quantitaUltimo");
      put("left_15", "quantitaTotale");
      put("left_16", "numeroContratti");

      put("right_1", "controvalore");
      put("right_2", "maxOggi");
      put("right_3", "maxAnno,maxAnnoDate");
      put("right_4", "minOggi");
      put("right_5", "minAnno,minAnnoDate");
      put("right_6", "chiusuraPrecedente");
      put("right_7", "prezzoRiferimento,dataOraPrezzoRifermento");
      put("right_8", "prezzoUfficiale,dataPrezzoUfficiale");
      put("right_9", "aperturaOdierna");
      put("right_10", "performance1Mese");
      put("right_11", "performance6Mesi");
      put("right_12", "performance1Anno");
    }
  };

  public static final HashMap<String, Class<?>> propToType = new HashMap<String, Class<?>>() {
    {
      // left table
      put("codiceIsin", String.class);
      put("idStrumento", Double.class);
      put("codiceAlfanumerico", String.class);
      put("superSector", String.class);
      put("mercatoSegmento", String.class);
      put("capitalizzazioneDiMercato", Double.class);
      put("lottoMinimo", Double.class);
      put("faseDiMercato", String.class);
      put("prezzoUltimoContratto", Double.class);
      put("varPercentuale", Double.class);
      put("varAssoluta", Double.class);
      put("prMedioProgr", Double.class);
      put("dataOraUltimoContratto", LocalDateTime.class);
      put("quantitaUltimo", Double.class);
      put("quantitaTotale", Double.class);
      put("numeroContratti", Integer.class);
      // right table
      put("controvalore", Double.class);
      put("maxOggi", Double.class);
      put("maxAnno", Double.class);
      put("maxAnnoDate", LocalDate.class);
      put("minOggi", Double.class);
      put("minAnno", Double.class);
      put("minAnnoDate", LocalDate.class);
      put("chiusuraPrecedente", Double.class);
      put("prezzoRiferimento", Double.class);
      put("dataOraPrezzoRifermento", LocalDateTime.class);
      put("prezzoUfficiale", Double.class);
      put("dataPrezzoUfficiale", LocalDate.class);
      put("aperturaOdierna", Double.class);
      put("performance1Mese", Double.class);
      put("performance6Mesi", Double.class);
      put("performance1Anno", Double.class);
    }
  };
}
