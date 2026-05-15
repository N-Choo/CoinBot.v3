import '../styles/homepage.css'
import HeroSection from '../components/landing/HeroSection'
import StatsSection from '../components/landing/StatsSection'
import ReactorSection from '../components/landing/ReactorSection'
import FeaturesSection from '../components/landing/FeaturesSection'
import FooterSection from '../components/landing/FooterSection'

export default function Landing() {
  return (
    <div className="landing">
      <HeroSection />
      <StatsSection />
      <ReactorSection />
      <FeaturesSection />
      <FooterSection />
    </div>
  )
}
