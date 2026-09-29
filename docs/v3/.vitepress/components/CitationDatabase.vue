<script setup>
import { ref, computed, onMounted } from 'vue'

const citations = ref([])
const searchQuery = ref('')
const authorQuery = ref('')
const selectedTopic = ref('')
const yearMin = ref(1885)
const yearMax = ref(2026)
const sortBy = ref('year-desc')
const selectedIds = ref(new Set())
const showExport = ref(false)
const exportText = ref('')
const isLoaded = ref(false)

const topicSections = [
  { id: '00', label: '00 -- Lifecycle and Finite Agency' },
  { id: '01', label: '01 -- Memory Consolidation' },
  { id: '02', label: '02 -- Affective Computing' },
  { id: '03', label: '03 -- Dreams and Offline Learning' },
  { id: '04', label: '04 -- Coordination and Multi-Agent' },
  { id: '05', label: '05 -- Biological Analogues' },
  { id: '06', label: '06 -- Self-Learning Systems' },
  { id: '07', label: '07 -- Context Engineering' },
  { id: '08', label: '08 -- Security and Provenance' },
  { id: '09', label: '09 -- HDC and Vector Symbolic Architectures' },
  { id: '10', label: '10 -- Market Microstructure' },
  { id: '11', label: '11 -- Streaming Algorithms' },
  { id: '12', label: '12 -- Signal Processing' },
  { id: '13', label: '13 -- Philosophy' },
  { id: '14', label: '14 -- Agent Harnesses and Tool Use' },
  { id: '15', label: '15 -- Cybernetics and VSM' },
  { id: '16', label: '16 -- Active Inference' },
  { id: '17', label: '17 -- Process Reward Models' },
  { id: '18', label: '18 -- Collective Intelligence' },
  { id: '19', label: '19 -- Regulatory Compliance' },
  { id: '20', label: '20 -- Cognitive Architectures' },
  { id: '21', label: '21 -- Mechanism Design' },
  { id: '22', label: '22 -- Protocol Standards' },
  { id: '23', label: '23 -- Generational and Evolutionary' },
  { id: '25', label: '25 -- Research to Runtime' },
  { id: '26', label: '26 -- Harness Engineering (2026)' },
  { id: '27', label: '27 -- Agent Benchmarks and Evaluation' },
  { id: '28', label: '28 -- Process Reward Models (2025-2026)' },
  { id: '29', label: '29 -- Forgetting and Memory Governance' },
  { id: '30', label: '30 -- Agent Deployment and Production' },
  { id: '33', label: '33 -- Category Theory and Cross-Cuts' },
]

onMounted(async () => {
  try {
    // Use VitePress base URL for correct path resolution
    const base = import.meta.env.BASE_URL || '/'
    const url = `${base}citations.json`.replace('//', '/')
    const response = await fetch(url)
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    citations.value = await response.json()
    isLoaded.value = true
  } catch (e) {
    // Fallback: try relative import at build time
    try {
      const mod = await import('../../citations.json')
      citations.value = mod.default || mod
      isLoaded.value = true
    } catch (e2) {
      console.error('Failed to load citations:', e2)
    }
  }
})

const filteredCitations = computed(() => {
  let results = citations.value

  // Topic filter
  if (selectedTopic.value) {
    results = results.filter(c => c.topics.includes(selectedTopic.value))
  }

  // Year range filter
  results = results.filter(c => c.year >= yearMin.value && c.year <= yearMax.value)

  // Author search
  if (authorQuery.value.trim()) {
    const q = authorQuery.value.trim().toLowerCase()
    results = results.filter(c =>
      c.authors.some(a => a.toLowerCase().includes(q))
    )
  }

  // Full-text search across title, annotation, and authors
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.trim().toLowerCase()
    results = results.filter(c =>
      c.title.toLowerCase().includes(q) ||
      c.annotation.toLowerCase().includes(q) ||
      c.authors.some(a => a.toLowerCase().includes(q)) ||
      (c.venue && c.venue.toLowerCase().includes(q))
    )
  }

  // Sort
  results = [...results]
  switch (sortBy.value) {
    case 'year-desc':
      results.sort((a, b) => b.year - a.year)
      break
    case 'year-asc':
      results.sort((a, b) => a.year - b.year)
      break
    case 'author':
      results.sort((a, b) => (a.authors[0] || '').localeCompare(b.authors[0] || ''))
      break
    case 'topic':
      results.sort((a, b) => (a.topics[0] || '').localeCompare(b.topics[0] || ''))
      break
  }

  return results
})

const stats = computed(() => {
  const topics = new Set()
  const years = []
  citations.value.forEach(c => {
    c.topics.forEach(t => topics.add(t))
    years.push(c.year)
  })
  return {
    total: citations.value.length,
    topics: topics.size,
    earliest: years.length ? Math.min(...years) : 0,
    latest: years.length ? Math.max(...years) : 0,
    filtered: filteredCitations.value.length,
  }
})

function toggleSelect(id) {
  const s = new Set(selectedIds.value)
  if (s.has(id)) {
    s.delete(id)
  } else {
    s.add(id)
  }
  selectedIds.value = s
}

function selectAll() {
  const s = new Set(selectedIds.value)
  filteredCitations.value.forEach(c => s.add(c.id))
  selectedIds.value = s
}

function deselectAll() {
  selectedIds.value = new Set()
}

function exportBibtex() {
  const selected = citations.value.filter(c => selectedIds.value.has(c.id))
  if (selected.length === 0) return

  const entries = selected.map(c => {
    const authorStr = c.authors.join(' and ')
    const key = c.id
    const fields = [
      `  author    = {${authorStr}}`,
      `  title     = {${c.title}}`,
      `  year      = {${c.year}}`,
    ]
    if (c.venue) fields.push(`  journal   = {${c.venue}}`)
    if (c.arxiv_id) fields.push(`  eprint    = {${c.arxiv_id}}`)
    if (c.url) fields.push(`  url       = {${c.url}}`)

    // Determine entry type
    let entryType = 'misc'
    if (c.venue && (c.venue.includes('arXiv') || c.arxiv_id)) {
      entryType = 'article'
    } else if (c.venue && (c.venue.includes('Press') || c.venue.includes('Books') || c.venue.includes('Springer') || c.venue.includes('MIT') || c.venue.includes('Wiley'))) {
      entryType = 'book'
    } else if (c.venue && (c.venue.includes('NeurIPS') || c.venue.includes('ICML') || c.venue.includes('ICLR') || c.venue.includes('AAAI') || c.venue.includes('AAMAS') || c.venue.includes('ACL') || c.venue.includes('STOC') || c.venue.includes('SIGMOD'))) {
      entryType = 'inproceedings'
    } else if (c.venue) {
      entryType = 'article'
    }

    return `@${entryType}{${key},\n${fields.join(',\n')}\n}`
  })

  exportText.value = entries.join('\n\n')
  showExport.value = true
}

function copyExport() {
  navigator.clipboard.writeText(exportText.value).then(() => {
    // brief visual feedback handled by button text swap
  })
}

function getArxivUrl(arxivId) {
  return `https://arxiv.org/abs/${arxivId}`
}

function getTopicLabel(topicId) {
  const t = topicSections.find(s => s.id === topicId)
  return t ? t.label : `Section ${topicId}`
}

function resetFilters() {
  searchQuery.value = ''
  authorQuery.value = ''
  selectedTopic.value = ''
  yearMin.value = 1885
  yearMax.value = 2026
  sortBy.value = 'year-desc'
}
</script>

<template>
  <div class="citation-database">
    <!-- Stats bar -->
    <div class="stats-bar" v-if="isLoaded">
      <span class="stat">
        <strong>{{ stats.total }}</strong> citations
      </span>
      <span class="stat">
        <strong>{{ stats.topics }}</strong> topic sections
      </span>
      <span class="stat">
        <strong>{{ stats.earliest }}</strong> &ndash; <strong>{{ stats.latest }}</strong>
      </span>
      <span class="stat" v-if="stats.filtered !== stats.total">
        <strong>{{ stats.filtered }}</strong> shown
      </span>
      <span class="stat" v-if="selectedIds.size > 0">
        <strong>{{ selectedIds.size }}</strong> selected
      </span>
    </div>

    <!-- Filters -->
    <div class="filters">
      <div class="filter-row">
        <div class="filter-group search-group">
          <label for="search-input">Search</label>
          <input
            id="search-input"
            v-model="searchQuery"
            type="text"
            placeholder="Search titles, annotations, venues..."
            class="filter-input"
          />
        </div>
        <div class="filter-group author-group">
          <label for="author-input">Author</label>
          <input
            id="author-input"
            v-model="authorQuery"
            type="text"
            placeholder="Author name..."
            class="filter-input"
          />
        </div>
      </div>

      <div class="filter-row">
        <div class="filter-group topic-group">
          <label for="topic-select">Topic Section</label>
          <select id="topic-select" v-model="selectedTopic" class="filter-select">
            <option value="">All sections</option>
            <option v-for="t in topicSections" :key="t.id" :value="t.id">
              {{ t.label }}
            </option>
          </select>
        </div>

        <div class="filter-group year-group">
          <label>Year Range</label>
          <div class="year-inputs">
            <input
              v-model.number="yearMin"
              type="number"
              min="1885"
              max="2026"
              class="filter-input year-input"
            />
            <span class="year-separator">&ndash;</span>
            <input
              v-model.number="yearMax"
              type="number"
              min="1885"
              max="2026"
              class="filter-input year-input"
            />
          </div>
        </div>

        <div class="filter-group sort-group">
          <label for="sort-select">Sort by</label>
          <select id="sort-select" v-model="sortBy" class="filter-select">
            <option value="year-desc">Year (newest)</option>
            <option value="year-asc">Year (oldest)</option>
            <option value="author">Author (A-Z)</option>
            <option value="topic">Topic section</option>
          </select>
        </div>
      </div>

      <div class="filter-actions">
        <button class="btn btn-secondary" @click="resetFilters">Reset filters</button>
        <button class="btn btn-secondary" @click="selectAll">Select all shown</button>
        <button class="btn btn-secondary" @click="deselectAll" v-if="selectedIds.size > 0">
          Deselect all
        </button>
        <button
          class="btn btn-primary"
          @click="exportBibtex"
          :disabled="selectedIds.size === 0"
        >
          Export BibTeX ({{ selectedIds.size }})
        </button>
      </div>
    </div>

    <!-- Export modal -->
    <div class="export-modal" v-if="showExport">
      <div class="export-header">
        <h3>BibTeX Export ({{ selectedIds.size }} entries)</h3>
        <div class="export-actions">
          <button class="btn btn-primary" @click="copyExport">Copy to clipboard</button>
          <button class="btn btn-secondary" @click="showExport = false">Close</button>
        </div>
      </div>
      <pre class="export-content"><code>{{ exportText }}</code></pre>
    </div>

    <!-- Results -->
    <div class="results" v-if="isLoaded">
      <div
        v-for="cite in filteredCitations"
        :key="cite.id"
        class="citation-card"
        :class="{ selected: selectedIds.has(cite.id) }"
      >
        <div class="citation-header">
          <label class="citation-checkbox">
            <input
              type="checkbox"
              :checked="selectedIds.has(cite.id)"
              @change="toggleSelect(cite.id)"
            />
          </label>
          <div class="citation-meta">
            <span class="citation-year">{{ cite.year }}</span>
            <span
              class="citation-topic"
              v-for="topic in cite.topics"
              :key="topic"
            >{{ topic }}</span>
          </div>
        </div>

        <div class="citation-body">
          <div class="citation-authors">{{ cite.authors.join(', ') }}</div>
          <div class="citation-title">{{ cite.title }}</div>
          <div class="citation-venue" v-if="cite.venue">
            {{ cite.venue }}
            <template v-if="cite.arxiv_id">
              &middot;
              <a :href="getArxivUrl(cite.arxiv_id)" target="_blank" rel="noopener">
                arXiv:{{ cite.arxiv_id }}
              </a>
            </template>
            <template v-else-if="cite.url">
              &middot;
              <a :href="cite.url" target="_blank" rel="noopener">Link</a>
            </template>
          </div>
          <div class="citation-annotation">{{ cite.annotation }}</div>
          <div class="citation-chapters" v-if="cite.chapters && cite.chapters.length">
            <span class="chapters-label">Chapters:</span>
            <span
              class="chapter-tag"
              v-for="ch in cite.chapters"
              :key="ch"
            >{{ ch }}</span>
          </div>
        </div>
      </div>

      <div class="no-results" v-if="filteredCitations.length === 0">
        No citations match your current filters. Try broadening your search.
      </div>
    </div>

    <div class="loading" v-else>
      Loading citation database...
    </div>
  </div>
</template>

<style scoped>
.citation-database {
  max-width: 900px;
  margin: 0 auto;
}

/* Stats bar */
.stats-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  margin-bottom: 1.5rem;
  font-size: 0.9rem;
  color: var(--vp-c-text-2);
}

.stat strong {
  color: var(--vp-c-brand-1);
}

/* Filters */
.filters {
  background: var(--vp-c-bg-soft);
  border-radius: 8px;
  padding: 1.25rem;
  margin-bottom: 1.5rem;
}

.filter-row {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.filter-row:last-child {
  margin-bottom: 0;
}

.filter-group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.filter-group label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--vp-c-text-2);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.search-group {
  flex: 2;
  min-width: 200px;
}

.author-group {
  flex: 1;
  min-width: 160px;
}

.topic-group {
  flex: 2;
  min-width: 200px;
}

.year-group {
  flex: 1;
  min-width: 160px;
}

.sort-group {
  flex: 1;
  min-width: 140px;
}

.filter-input,
.filter-select {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  font-size: 0.9rem;
  width: 100%;
  box-sizing: border-box;
  transition: border-color 0.2s;
}

.filter-input:focus,
.filter-select:focus {
  outline: none;
  border-color: var(--vp-c-brand-1);
}

.year-inputs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.year-input {
  width: 80px !important;
  text-align: center;
}

.year-separator {
  color: var(--vp-c-text-3);
}

.filter-actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-top: 0.75rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--vp-c-divider);
}

/* Buttons */
.btn {
  padding: 0.4rem 0.85rem;
  border-radius: 6px;
  font-size: 0.85rem;
  font-weight: 500;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.2s;
}

.btn-primary {
  background: var(--vp-c-brand-1);
  color: var(--vp-c-white);
  border-color: var(--vp-c-brand-1);
}

.btn-primary:hover:not(:disabled) {
  background: var(--vp-c-brand-2);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background: var(--vp-c-bg);
  color: var(--vp-c-text-1);
  border-color: var(--vp-c-divider);
}

.btn-secondary:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}

/* Export modal */
.export-modal {
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
}

.export-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.export-header h3 {
  margin: 0;
  font-size: 1rem;
  color: var(--vp-c-text-1);
}

.export-actions {
  display: flex;
  gap: 0.5rem;
}

.export-content {
  max-height: 400px;
  overflow-y: auto;
  padding: 1rem;
  background: var(--vp-c-bg);
  border-radius: 6px;
  border: 1px solid var(--vp-c-divider);
  font-size: 0.8rem;
  line-height: 1.5;
  margin: 0;
}

.export-content code {
  font-family: var(--vp-font-family-mono);
  white-space: pre;
}

/* Citation cards */
.citation-card {
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  padding: 1rem 1.25rem;
  margin-bottom: 0.75rem;
  background: var(--vp-c-bg);
  transition: border-color 0.2s, box-shadow 0.2s;
}

.citation-card:hover {
  border-color: var(--vp-c-brand-soft);
}

.citation-card.selected {
  border-color: var(--vp-c-brand-1);
  box-shadow: 0 0 0 1px var(--vp-c-brand-soft);
}

.citation-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}

.citation-checkbox input {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: var(--vp-c-brand-1);
}

.citation-meta {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  flex-wrap: wrap;
}

.citation-year {
  font-size: 0.85rem;
  font-weight: 700;
  color: var(--vp-c-brand-1);
  font-variant-numeric: tabular-nums;
}

.citation-topic {
  font-size: 0.7rem;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.citation-body {
  padding-left: calc(16px + 0.75rem);
}

.citation-authors {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--vp-c-text-1);
  margin-bottom: 0.15rem;
}

.citation-title {
  font-size: 0.9rem;
  font-style: italic;
  color: var(--vp-c-text-1);
  margin-bottom: 0.25rem;
  line-height: 1.4;
}

.citation-venue {
  font-size: 0.8rem;
  color: var(--vp-c-text-2);
  margin-bottom: 0.4rem;
}

.citation-venue a {
  color: var(--vp-c-brand-1);
  text-decoration: none;
}

.citation-venue a:hover {
  text-decoration: underline;
}

.citation-annotation {
  font-size: 0.85rem;
  color: var(--vp-c-text-2);
  line-height: 1.5;
  margin-bottom: 0.4rem;
}

.citation-chapters {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
  align-items: center;
}

.chapters-label {
  font-size: 0.75rem;
  color: var(--vp-c-text-3);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.chapter-tag {
  font-size: 0.7rem;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-2);
  border: 1px solid var(--vp-c-divider);
  font-family: var(--vp-font-family-mono);
}

.no-results {
  text-align: center;
  padding: 3rem 1rem;
  color: var(--vp-c-text-3);
  font-size: 0.95rem;
}

.loading {
  text-align: center;
  padding: 3rem 1rem;
  color: var(--vp-c-text-3);
}

/* Responsive */
@media (max-width: 640px) {
  .filter-row {
    flex-direction: column;
  }

  .citation-body {
    padding-left: 0;
  }

  .year-input {
    width: 60px !important;
  }

  .filter-actions {
    flex-direction: column;
  }

  .filter-actions .btn {
    width: 100%;
    text-align: center;
  }
}
</style>
