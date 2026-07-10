/**
 * Phase 8 type shape tests.
 *
 * RED: Fails until types.ts exports ExtractedEntity / ExtractedEntities /
 * ExtractionSettings and mock-data.ts exports mockExtractionSettings +
 * mockDocumentEntitiesContainer with the Phase 8 entity shapes.
 */
import { describe, it, expect } from "vitest";
import { mockExtractionSettings, mockDocumentEntitiesContainer, mockDocuments } from "./mock-data";
import type {
  ExtractedEntity,
  ExtractedEntities,
  ExtractionSettings,
  Settings,
  PredicateEntry,
  EntitySubclass,
  ConsolidationKind,
  OntologyStoreSchema,
  PromoteResult,
} from "./types";

describe("Phase 8: mock-data exports", () => {
  it("mockExtractionSettings is exported with correct shape", () => {
    expect(mockExtractionSettings).toBeDefined();
    expect(typeof (mockExtractionSettings as any).extractionModel).toBe("string");
    expect(typeof (mockExtractionSettings as any).useLlmExtraction).toBe("boolean");
  });

  it("mockExtractionSettings.extractionModel defaults to empty string", () => {
    expect((mockExtractionSettings as any).extractionModel).toBe("");
  });

  it("mockExtractionSettings.useLlmExtraction defaults to true", () => {
    expect((mockExtractionSettings as any).useLlmExtraction).toBe(true);
  });

  it("mockDocumentEntitiesContainer is exported with all required fields", () => {
    expect(mockDocumentEntitiesContainer).toBeDefined();
    expect(Array.isArray((mockDocumentEntitiesContainer as any).entities)).toBe(true);
    expect((mockDocumentEntitiesContainer as any).topic).toBe("identity");
    expect(Array.isArray((mockDocumentEntitiesContainer as any).tags)).toBe(true);
    expect(typeof (mockDocumentEntitiesContainer as any).entitiesVersion).toBe("number");
    expect((mockDocumentEntitiesContainer as any).language).toBe("en");
  });

  it("mockDocumentEntitiesContainer.entitiesVersion is 3", () => {
    expect((mockDocumentEntitiesContainer as any).entitiesVersion).toBe(3);
  });

  it("mockDocuments[0].extractedEntities has at least one entity with confidence < 0.7 (Also found expander data)", () => {
    const entities = mockDocuments[0].extractedEntities;
    const hasLowConfidence = entities.some(
      (e: any) => typeof e.confidence === "number" && e.confidence < 0.7,
    );
    expect(hasLowConfidence).toBe(true);
  });

  it("mockDocuments[0].extractedEntities has at least one entity with class field", () => {
    const entities = mockDocuments[0].extractedEntities;
    const hasClass = entities.some((e: any) => typeof e.class === "string");
    expect(hasClass).toBe(true);
  });

  it("mockDocuments[0].extractedEntities has an Identifier entity with subclass", () => {
    const entities = mockDocuments[0].extractedEntities;
    const identifier = entities.find((e: any) => e.class === "Identifier" && e.subclass);
    expect(identifier).toBeDefined();
  });
});

describe("Phase 8: ExtractedEntity type shape (compile-time verification)", () => {
  it("ExtractedEntity accepts Phase 6 shape (label, value, entityType)", () => {
    const entity: ExtractedEntity = {
      label: "Year",
      value: "2025",
      entityType: "date",
    };
    expect(entity.label).toBe("Year");
    // optional Phase 8 fields should be absent
    expect(entity.class).toBeUndefined();
    expect(entity.confidence).toBeUndefined();
  });

  it("ExtractedEntity accepts Phase 8 shape with optional class/subclass/confidence", () => {
    const entity: ExtractedEntity = {
      label: "Aadhaar",
      value: "1234 5678 9012",
      entityType: "identifier",
      class: "Identifier",
      subclass: "aadhaar",
      confidence: 0.92,
    };
    expect(entity.class).toBe("Identifier");
    expect(entity.subclass).toBe("aadhaar");
    expect(entity.confidence).toBe(0.92);
  });
});

describe("Phase 8: ExtractionSettings type shape (compile-time verification)", () => {
  it("ExtractionSettings can be constructed with extractionModel and useLlmExtraction", () => {
    const settings: ExtractionSettings = {
      extractionModel: "claude-haiku-4-5-20251001",
      useLlmExtraction: true,
    };
    expect(settings.extractionModel).toBe("claude-haiku-4-5-20251001");
    expect(settings.useLlmExtraction).toBe(true);
  });
});

describe("Phase 8: ExtractedEntities container type (compile-time verification)", () => {
  it("ExtractedEntities can be constructed with all required fields", () => {
    const container: ExtractedEntities = {
      entities: [],
      topic: "identity",
      tags: ["aadhaar", "personal_id"],
      entitiesVersion: 3,
      language: "en",
    };
    expect(container.entitiesVersion).toBe(3);
    expect(container.language).toBe("en");
  });

  it("ExtractedEntities allows null topic and null language", () => {
    const container: ExtractedEntities = {
      entities: [],
      topic: null,
      tags: [],
      entitiesVersion: 3,
      language: null,
    };
    expect(container.topic).toBeNull();
    expect(container.language).toBeNull();
  });
});

describe("Phase 8: Settings type extension (compile-time verification)", () => {
  it("Settings type includes extractionModel and useLlmExtraction", () => {
    const settings: Settings = {
      theme: "dark",
      sidebarCollapsed: false,
      embeddingModel: "local",
      watchedFolders: [],
      excludedPatterns: [],
      indexOnStartup: true,
      indexSize: 0,
      storagePath: "/tmp",
      extractionModel: "claude-haiku-4-5-20251001",
      useLlmExtraction: true,
    };
    expect(settings.extractionModel).toBe("claude-haiku-4-5-20251001");
    expect(settings.useLlmExtraction).toBe(true);
  });
});

describe("Phase 11.6: Adaptive Ontology type shapes (compile-time verification)", () => {
  it("PredicateEntry / EntitySubclass shapes compile-check", () => {
    const predicate: PredicateEntry = {
      name: "registered_to",
      description: "Vehicle registered to a person",
      source: "adaptive",
      count: 3,
    };
    const subclass: EntitySubclass = {
      class: "Location",
      subclass: "apartment",
      source: "corpus",
      count: 1,
    };
    expect(predicate.name).toBe("registered_to");
    expect(subclass.class).toBe("Location");
  });

  it("ConsolidationKind discriminated union narrows on kind", () => {
    const merge: ConsolidationKind = { kind: "merge", from: ["a", "b"], into: "c" };
    const rename: ConsolidationKind = { kind: "rename", from: "old", to: "new" };
    if (merge.kind === "merge") {
      expect(merge.from.length).toBe(2);
    }
    if (rename.kind === "rename") {
      expect(rename.to).toBe("new");
    }
  });

  it("OntologyStoreSchema default shape parses", () => {
    const json = JSON.stringify({
      version: 1,
      corpusSeed: null,
      adaptivePredicates: [],
      pendingPredicates: [],
      manualPredicates: [],
      entitySubclasses: [],
      pendingConsolidation: null,
      automaticGrowthEnabled: false,
      bootstrapCompletedAt: null,
      lastConsolidationAt: null,
      triplesSinceLastConsolidation: 0,
    });
    const schema: OntologyStoreSchema = JSON.parse(json);
    expect(schema.version).toBe(1);
  });

  it("PromoteResult narrows", () => {
    const result: PromoteResult = { kind: "stillpending", count: 1 };
    if (result.kind === "stillpending") {
      expect(result.count).toBe(1);
    }
  });
});
