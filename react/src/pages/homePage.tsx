import '../styles/homepage.css'
import HeroSection from '../components/landing/HeroSection'
import StatsBar from '../components/landing/StatsBar'
import ActivityFeed from '../components/landing/ActivityFeed'
import FeaturesSection from '../components/landing/FeaturesSection'
import FooterSection from '../components/landing/FooterSection'

export default function Landing() {
  return (
    <div className="landing">
      <HeroSection />
      <StatsBar />
      <ActivityFeed />
      <FeaturesSection />
      <FooterSection />
    </div>
  )
}
