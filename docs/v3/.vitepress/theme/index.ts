import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import { h, defineAsyncComponent } from 'vue'
import './custom.css'

// Async load heavy components (Three.js) to avoid SSR issues
const SpectreHero = defineAsyncComponent(() =>
  import('../components/SpectreHero.vue')
)
const ParticleBackground = defineAsyncComponent(() =>
  import('../components/ParticleBackground.vue')
)
const CrateGraph3D = defineAsyncComponent(() =>
  import('../components/CrateGraph3D.vue')
)
const PipelineFlow3D = defineAsyncComponent(() =>
  import('../components/PipelineFlow3D.vue')
)
const SignalLifecycle3D = defineAsyncComponent(() =>
  import('../components/SignalLifecycle3D.vue')
)

// Sync load lightweight components
import CitationRef from '../components/CitationRef.vue'
import CrateLink from '../components/CrateLink.vue'
import InteractiveDiagram from '../components/InteractiveDiagram.vue'
import CitationDatabase from '../components/CitationDatabase.vue'

export default {
  extends: DefaultTheme,
  Layout() {
    return h(DefaultTheme.Layout, null, {
      'layout-bottom': () => h(ParticleBackground)
    })
  },
  enhanceApp({ app }) {
    app.component('SpectreHero', SpectreHero)
    app.component('ParticleBackground', ParticleBackground)
    app.component('CrateGraph3D', CrateGraph3D)
    app.component('PipelineFlow3D', PipelineFlow3D)
    app.component('SignalLifecycle3D', SignalLifecycle3D)
    app.component('CitationRef', CitationRef)
    app.component('CrateLink', CrateLink)
    app.component('InteractiveDiagram', InteractiveDiagram)
    app.component('CitationDatabase', CitationDatabase)
  },
} satisfies Theme
