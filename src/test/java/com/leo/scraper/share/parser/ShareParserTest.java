package com.leo.scraper.share.parser;

import static org.junit.jupiter.api.Assertions.*;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import java.io.IOException;

import com.leo.scraper.share.Share;

public class ShareParserTest {

  private static Share testShare;

  @BeforeAll
  public static void setUp() throws IOException {
    testShare = new Share("IT0003796171");
  }

  @Test
  public void testScrapeISINSetsPropertiesCorrectly() {
    assertNotNull(testShare.getCodiceIsin());
    assertNotNull(testShare.getIdStrumento());
    assertNotNull(testShare.getCodiceAlfanumerico());
    assertNotNull(testShare.getSuperSector());
    assertNotNull(testShare.getMercatoSegmento());
    assertNotNull(testShare.getCapitalizzazioneDiMercato());
    assertNotNull(testShare.getLottoMinimo());
    assertNotNull(testShare.getFaseDiMercato());
    assertNotNull(testShare.getPrezzoUltimoContratto());
    assertNotNull(testShare.getVarPercentuale());
    assertNotNull(testShare.getVarAssoluta());
    assertNotNull(testShare.getPrMedioProgr());
    assertNotNull(testShare.getDataOraUltimoContratto());
    assertNotNull(testShare.getQuantitaUltimo());
    assertNotNull(testShare.getQuantitaTotale());
    assertNotNull(testShare.getNumeroContratti());
    assertNotNull(testShare.getControvalore());
    assertNotNull(testShare.getMaxOggi());
    assertNotNull(testShare.getMinOggi());
    assertNotNull(testShare.getChiusuraPrecedente());
    assertNotNull(testShare.getAperturaOdierna());
  }

  @Test
  public void testPerformanceReferences() {
    assertNotNull(testShare.getPerformance1Mese());
    assertNotNull(testShare.getPerformance6Mesi());
    assertNotNull(testShare.getPerformance1Anno());
  }

  @Test
  public void testPriceDateReferences() {
    assertNotNull(testShare.getMinAnno());
    assertNotNull(testShare.getMinAnnoDate());
    assertNotNull(testShare.getMaxAnno());
    assertNotNull(testShare.getMaxAnnoDate());
    assertNotNull(testShare.getPrezzoRiferimento());
    assertNotNull(testShare.getDataOraPrezzoRifermento());
    assertNotNull(testShare.getPrezzoUfficiale());
    assertNotNull(testShare.getDataPrezzoUfficiale());
  }
}
