/**
 * json-ui Component Catalog
 *
 * Zod schemas for json-render components
 */

import { z } from 'zod';

// ============================================
// Base Types
// ============================================

/** Supports both plain string and { en, zh } i18n object */
export const I18nString = z.union([
  z.string(),
  z.object({ en: z.string(), zh: z.string() }),
]);

export type I18nStringType = z.infer<typeof I18nString>;

export const IconType = z.enum([
  'paper', 'user', 'calendar', 'tag', 'link', 'code',
  'chart', 'bulb', 'check', 'star', 'warning', 'info',
  'github', 'arxiv', 'pdf', 'copy', 'expand', 'collapse',
]);

export const ThemeType = z.enum(['light', 'dark', 'auto']);

export const VariantType = z.enum(['default', 'primary', 'success', 'warning', 'danger']);

export const SizeType = z.enum(['sm', 'md', 'lg']);

// ============================================
// Layout Components
// ============================================

export const ReportSchema = z.object({
  title: I18nString.optional(),
  theme: ThemeType.default('auto'),
  showLanguageSwitcher: z.boolean().optional(),
});

export const SectionSchema = z.object({
  title: I18nString,
  icon: IconType.optional(),
  collapsible: z.boolean().default(false),
  defaultExpanded: z.boolean().default(true),
});

export const GridSchema = z.object({
  cols: z.union([z.number(), z.record(z.string(), z.number())]).default(1),
  gap: SizeType.default('md'),
});

export const CardSchema = z.object({
  variant: VariantType.default('default'),
  padding: z.enum(['none', 'sm', 'md', 'lg']).default('md'),
  shadow: z.boolean().default(true),
});

// ============================================
// Paper Info Components
// ============================================

export const PaperHeaderSchema = z.object({
  title: I18nString,
  arxivId: z.string(),
  date: z.string(),
  categories: z.array(z.string()).optional(),
  version: z.string().optional(),
});

export const AuthorSchema = z.object({
  name: z.string(),
  affiliation: z.string().optional(),
  email: z.string().optional(),
  orcid: z.string().optional(),
});

export const AuthorListSchema = z.object({
  authors: z.array(AuthorSchema),
  layout: z.enum(['inline', 'list', 'grid']).default('inline'),
  showAffiliations: z.boolean().default(true),
  maxVisible: z.number().optional(),
});

export const AbstractSchema = z.object({
  text: I18nString,
  highlights: z.array(z.string()).optional(),
  maxLength: z.number().optional(),
});

export const TagSchema = z.object({
  label: I18nString,
  color: z.string().optional(),
  href: z.string().optional(),
});

export const TagListSchema = z.object({
  tags: z.array(TagSchema),
  variant: z.enum(['solid', 'outline', 'subtle']).default('subtle'),
});

// ============================================
// Content Components
// ============================================

export const KeyPointSchema = z.object({
  icon: IconType.default('bulb'),
  title: I18nString,
  description: I18nString,
  variant: VariantType.default('default'),
});

export const ContributionItemSchema = z.object({
  title: I18nString,
  description: I18nString.optional(),
  badge: I18nString.optional(),
});

export const ContributionListSchema = z.object({
  items: z.array(ContributionItemSchema),
  numbered: z.boolean().default(true),
});

export const MethodStepSchema = z.object({
  step: z.number(),
  title: I18nString,
  description: I18nString,
  icon: IconType.optional(),
});

export const MethodOverviewSchema = z.object({
  steps: z.array(MethodStepSchema),
  showConnectors: z.boolean().default(true),
});

export const HighlightSchema = z.object({
  text: I18nString,
  type: z.enum(['quote', 'important', 'warning', 'code']).default('quote'),
  source: I18nString.optional(),
});

export const CodeBlockSchema = z.object({
  code: z.string(),
  language: z.string().default('text'),
  title: I18nString.optional(),
  showLineNumbers: z.boolean().default(false),
});

// ============================================
// Data Display Components
// ============================================

export const MetricSchema = z.object({
  label: I18nString,
  value: z.union([z.string(), z.number()]),
  previousValue: z.union([z.string(), z.number()]).optional(),
  trend: z.enum(['up', 'down', 'neutral']).optional(),
  format: z.enum(['number', 'percent', 'currency', 'date']).optional(),
  suffix: z.string().optional(),
  icon: IconType.optional(),
});

export const MetricsGridSchema = z.object({
  metrics: z.array(MetricSchema),
  cols: z.number().default(4),
});

export const TableColumnSchema = z.object({
  key: z.string(),
  label: I18nString,
  align: z.enum(['left', 'center', 'right']).default('left'),
  width: z.string().optional(),
});

export const TableSchema = z.object({
  columns: z.array(TableColumnSchema),
  rows: z.array(z.record(z.string(), z.unknown())),
  striped: z.boolean().default(true),
  compact: z.boolean().default(false),
  caption: I18nString.optional(),
});

// ============================================
// Rich Content Components
// ============================================

export const ImageSchema = z.object({
  src: z.string(),
  alt: I18nString.optional(),
  caption: I18nString.optional(),
  width: z.string().optional(), // e.g., "100%", "600px"
});

export const FigureSchema = z.object({
  images: z.array(ImageSchema),
  caption: I18nString.optional(),
  label: I18nString.optional(), // e.g., "Figure 1"
});

export const FormulaSchema = z.object({
  latex: z.string(),
  block: z.boolean().default(false), // display mode
  label: z.string().optional(),
});

export const ProseSchema = z.object({
  content: I18nString, // Markdown content
});

export const CalloutSchema = z.object({
  type: z.enum(['info', 'tip', 'warning', 'important', 'note']).default('info'),
  title: I18nString.optional(),
  content: I18nString,
});

export const DefinitionSchema = z.object({
  term: I18nString,
  definition: I18nString,
});

export const DefinitionListSchema = z.object({
  items: z.array(DefinitionSchema),
});

export const TheoremSchema = z.object({
  type: z.enum(['theorem', 'lemma', 'proposition', 'corollary', 'definition', 'remark']).default('theorem'),
  number: z.string().optional(),
  title: I18nString.optional(),
  content: I18nString,
});

export const AlgorithmSchema = z.object({
  title: I18nString,
  steps: z.array(z.object({
    line: z.number(),
    code: z.string(),
    indent: z.number().default(0),
  })),
  caption: I18nString.optional(),
});

export const ResultsTableSchema = z.object({
  caption: I18nString.optional(),
  columns: z.array(z.object({
    key: z.string(),
    label: I18nString,
    highlight: z.boolean().default(false), // highlight best result
  })),
  rows: z.array(z.record(z.string(), z.unknown())),
  highlights: z.array(z.object({
    row: z.number(),
    col: z.string(),
  })).optional(), // cells to highlight as best
});

// ============================================
// Interactive Components
// ============================================

export const LinkButtonSchema = z.object({
  href: z.string(),
  label: I18nString,
  icon: IconType.optional(),
  variant: VariantType.default('default'),
  external: z.boolean().default(true),
});

export const LinkGroupSchema = z.object({
  links: z.array(LinkButtonSchema),
  layout: z.enum(['horizontal', 'vertical']).default('horizontal'),
});

// ============================================
// Brand Components
// ============================================

export const BrandHeaderSchema = z.object({
  badge: I18nString.default('🤖 AI Generated Content'),
  poweredBy: I18nString.default('ActionBook'),
  showBadge: z.boolean().default(true),
});

export const BrandFooterSchema = z.object({
  timestamp: z.string(),
  attribution: I18nString.default('Powered by ActionBook'),
  disclaimer: I18nString.optional(),
});

// ============================================
// Component Catalog
// ============================================

export const catalog = {
  // Layout
  Report: ReportSchema,
  Section: SectionSchema,
  Grid: GridSchema,
  Card: CardSchema,

  // Paper Info
  PaperHeader: PaperHeaderSchema,
  AuthorList: AuthorListSchema,
  Abstract: AbstractSchema,
  TagList: TagListSchema,

  // Content
  KeyPoint: KeyPointSchema,
  ContributionList: ContributionListSchema,
  MethodOverview: MethodOverviewSchema,
  Highlight: HighlightSchema,
  CodeBlock: CodeBlockSchema,

  // Rich Content
  Image: ImageSchema,
  Figure: FigureSchema,
  Formula: FormulaSchema,
  Prose: ProseSchema,
  Callout: CalloutSchema,
  DefinitionList: DefinitionListSchema,
  Theorem: TheoremSchema,
  Algorithm: AlgorithmSchema,
  ResultsTable: ResultsTableSchema,

  // Data Display
  Metric: MetricSchema,
  MetricsGrid: MetricsGridSchema,
  Table: TableSchema,

  // Interactive
  LinkButton: LinkButtonSchema,
  LinkGroup: LinkGroupSchema,

  // Brand
  BrandHeader: BrandHeaderSchema,
  BrandFooter: BrandFooterSchema,
} as const;

export type CatalogType = typeof catalog;

// ============================================
// Type Exports
// ============================================

export type Icon = z.infer<typeof IconType>;
export type Theme = z.infer<typeof ThemeType>;
export type Variant = z.infer<typeof VariantType>;

export type ReportProps = z.infer<typeof ReportSchema>;
export type SectionProps = z.infer<typeof SectionSchema>;
export type PaperHeaderProps = z.infer<typeof PaperHeaderSchema>;
export type AuthorListProps = z.infer<typeof AuthorListSchema>;
export type AbstractProps = z.infer<typeof AbstractSchema>;
export type ContributionListProps = z.infer<typeof ContributionListSchema>;
export type MethodOverviewProps = z.infer<typeof MethodOverviewSchema>;
export type MetricProps = z.infer<typeof MetricSchema>;
export type MetricsGridProps = z.infer<typeof MetricsGridSchema>;
export type TableProps = z.infer<typeof TableSchema>;
export type LinkGroupProps = z.infer<typeof LinkGroupSchema>;
export type BrandHeaderProps = z.infer<typeof BrandHeaderSchema>;
export type BrandFooterProps = z.infer<typeof BrandFooterSchema>;
