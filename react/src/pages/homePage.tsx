import HeroSection from '../components/landing/HeroSection'
import StatsBar from '../components/landing/StatsBar'
import NewsFeed from '../components/landing/NewsFeed'
import FeaturesSection from '../components/landing/FeaturesSection'
import FooterSection from '../components/landing/FooterSection'

export default function Landing() {
  return (
    <div className="bg-bg-dark text-text-main min-h-screen overflow-x-hidden">
      <HeroSection />
      <StatsBar />
      <NewsFeed />
      <FeaturesSection />
      <FooterSection />
    </div>
  )
}
