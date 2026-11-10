export interface HowToStep {
  title: string
  body: string
}

export interface HowToFaq {
  question: string
  answer: string
}

export interface HowToGuide {
  slug: string
  eyebrow: string
  title: string
  intro: string
  requirements: string[]
  steps: HowToStep[]
  notes: string[]
  faqs: HowToFaq[]
}

export const GUIDES: Record<string, HowToGuide> = {
  deposit: {
    slug: 'deposit',
    eyebrow: 'Funding',
    title: 'How to deposit USDT',
    intro:
      'Deposits are USDT on Ethereum (ERC20). Send from the wallet you use to sign in; your balance is credited after the transaction and KuCoin confirmation.',
    requirements: [
      'A Web3 wallet such as MetaMask',
      'USDT on Ethereum (ERC20), not another chain or token',
      'A small amount of ETH for the network gas fee',
      'The wallet connected to CoinBot',
    ],
    steps: [
      {
        title: 'Connect your wallet',
        body: 'Use Connect in the top bar and sign the login message. This creates your account from your wallet address — there is no password or email.',
      },
      {
        title: 'Open Deposit Funds',
        body: 'Go to Dashboard and choose Deposit Funds. The dialog shows the platform deposit address.',
      },
      {
        title: 'Copy the platform address',
        body: 'Copy the address shown in the dialog. It is the platform’s KuCoin deposit address. Do not reuse an address from another exchange or chain.',
      },
      {
        title: 'Send USDT (ERC20)',
        body: 'From your connected wallet, send USDT on Ethereum to the copied address. Double-check the network — an unsupported asset or chain can be lost permanently.',
      },
      {
        title: 'Wait for confirmations',
        body: 'The transaction needs Ethereum confirmations and KuCoin confirmation. Your activity shows the deposit as pending until it is confirmed.',
      },
      {
        title: 'Balance credited',
        body: 'Once confirmed, the USDT is credited to your available balance and swept into the margin account that the trading engine uses.',
      },
    ],
    notes: [
      'Only USDT on Ethereum (ERC20) is supported. Sending another token or using another network can result in permanent loss.',
      'Network gas is paid by you. The platform does not charge a deposit fee.',
      'Deposits require on-chain confirmations and a KuCoin confirmation, so they usually take a few minutes.',
      'Always copy the address from the app. Do not reuse an address from an old message or another platform.',
    ],
    faqs: [
      { question: 'How long does a deposit take?', answer: 'It usually takes a few minutes: first Ethereum confirmations, then KuCoin credits the deposit.' },
      { question: 'Can I deposit another token?', answer: 'No. Only USDT on Ethereum (ERC20) is supported today.' },
      { question: 'Do I need ETH?', answer: 'Yes. You pay the Ethereum network gas fee from your own wallet.' },
      { question: 'Why is my balance not updated?', answer: 'The deposit may still be waiting for confirmations. Check the pending deposit in your activity, then refresh after the transaction confirms.' },
    ],
  },
  'buy-contract': {
    slug: 'buy-contract',
    eyebrow: 'Trading',
    title: 'How to buy a trading contract',
    intro:
      'Buying a contract allocates part of your USDT balance to the automated engine for a defined risk. You set the amount, stop, and target; the engine handles execution.',
    requirements: [
      'A connected wallet with available USDT',
      'A USDT trading pair such as BTC/USDT',
      'A stop loss and take profit you are comfortable with',
      'An understanding of the maximum loss you are committing',
    ],
    steps: [
      {
        title: 'Connect and open Trading',
        body: 'Connect your wallet in the top bar, then open Trading.',
      },
      {
        title: 'Choose a USDT pair',
        body: 'Select the market you want the engine to trade. Only /USDT pairs are supported.',
      },
      {
        title: 'Set the contract amount',
        body: 'The amount is the capital committed to the contract — and the maximum you can lose.',
      },
      {
        title: 'Set stop loss and take profit',
        body: 'Choose percentages for both. Stop loss must be below take profit.',
      },
      {
        title: 'Review the risk',
        body: 'Check the amount locked, the 5% performance fee on net profit, and the resulting target. Nothing is charged until the contract settles positive.',
      },
      {
        title: 'Sign the contract',
        body: 'Sign the message in your wallet. The amount is locked and the contract becomes active.',
      },
      {
        title: 'The engine trades',
        body: 'The engine waits for a validated signal, sizes the order to the contract, and executes on KuCoin cross-margin.',
      },
    ],
    notes: [
      'You do not choose long or short manually. The engine can open either direction based on the validated signal.',
      'The committed amount is the maximum loss; positions are sized to it.',
      'The 5% fee applies only to net profit after trading and borrow costs.',
      'The contract stays active until you close it or the engine exits on take profit or stop loss.',
    ],
    faqs: [
      { question: 'What does “buying a contract” mean?', answer: 'It allocates funds to the automated strategy with a fixed risk. It is not a spot purchase of the underlying coin.' },
      { question: 'Can I lose more than the contract amount?', answer: 'No. The engine sizes positions to the committed fund and caps realized loss at that amount.' },
      { question: 'How long does a contract last?', answer: 'Until you close it or the engine exits on take profit or stop loss.' },
      { question: 'Can I close early?', answer: 'Yes. See “How to sell or close a contract”.' },
    ],
  },
  'sell-contract': {
    slug: 'sell-contract',
    eyebrow: 'Settlement',
    title: 'How to sell or close a contract',
    intro:
      'Selling a contract means closing it and settling at market. The engine closes open positions, repays borrow interest, applies the 5% fee only on net profit, and returns the remaining funds to your balance.',
    requirements: [
      'An active contract owned by your wallet',
      'A connected wallet',
    ],
    steps: [
      {
        title: 'Open Active Contracts',
        body: 'Go to Dashboard or Trading and find the contract you want to close.',
      },
      {
        title: 'Select Close Contract',
        body: 'Choose Close Contract on that row. The contract moves to a closing state.',
      },
      {
        title: 'The engine closes positions',
        body: 'It market-closes any open positions for the contract. This is not a limit order, so the exit is at market.',
      },
      {
        title: 'Interest and fees settle',
        body: 'Borrow interest is repaid from the contract. The 5% performance fee is charged only if the contract ends positive, after costs.',
      },
      {
        title: 'Funds return',
        body: 'The remaining balance is credited to your account. You can withdraw it or start a new contract.',
      },
    ],
    notes: [
      'Closing settles the whole contract; partial closes are not supported.',
      'Settlement is asynchronous. It can take a few minutes if positions or borrow debt are still settling.',
      'There is no separate close fee. The only platform fee is 5% of net profit at settlement.',
      'The engine can also close automatically when take profit or stop loss is hit.',
    ],
    faqs: [
      { question: 'What does selling a contract do?', answer: 'It closes the contract, exits open positions, settles interest, and returns the remaining funds.' },
      { question: 'When do I receive the funds?', answer: 'After positions close and any borrow debt is repaid. The activity view shows the settlement status.' },
      { question: 'Is there a fee to close?', answer: 'No close fee. The platform charges 5% of net profit at settlement.' },
      { question: 'Can I sell part of a contract?', answer: 'No. A contract settles as a whole.' },
    ],
  },
  withdraw: {
    slug: 'withdraw',
    eyebrow: 'Funding',
    title: 'How to withdraw USDT',
    intro:
      'Withdraw available USDT to the wallet you used to sign in. You sign every withdrawal, and the destination and network fee are shown before you confirm.',
    requirements: [
      'A connected wallet with available (unlocked) USDT',
      'Enough balance for the amount plus the network fee',
      'The same wallet you use to sign in — withdrawals are self-wallet only',
    ],
    steps: [
      {
        title: 'Connect and open Dashboard',
        body: 'Connect your wallet and open Dashboard.',
      },
      {
        title: 'Select Withdraw Assets',
        body: 'The dialog shows the destination, network, fee, minimum, available balance, and maximum.',
      },
      {
        title: 'Review the destination',
        body: 'The destination is always your authenticated wallet. Arbitrary addresses are not supported.',
      },
      {
        title: 'Enter the amount you receive',
        body: 'The total debited is the amount plus the network fee. The fee is charged on top.',
      },
      {
        title: 'Sign the withdrawal',
        body: 'Sign the EIP-712 withdrawal message in your wallet. The request is queued with your balance locked.',
      },
      {
        title: 'Track the status',
        body: 'Follow pending → processing → submitted → completed in your activity. The tx hash appears once the exchange sends the funds.',
      },
    ],
    notes: [
      'The fee is on top: enter what you want to receive, and the total debited is amount + fee.',
      'The minimum and fee come live from KuCoin (for example, min 30 USDT and a 5.5 USDT ERC20 fee). They can change.',
      'Signed requests expire after five minutes. Expired or failed requests release the locked funds.',
      'Funds may need to move from the exchange margin account to the funding account first. The worker does this automatically.',
      'Withdrawals pause automatically if platform liabilities exceed free exchange funds.',
    ],
    faqs: [
      { question: 'Why is the amount received less than my balance?', answer: 'The network fee is charged on top. The amount you enter is what you receive.' },
      { question: 'Can I withdraw to another address?', answer: 'No. Withdrawals go only to the wallet that authenticated the session.' },
      { question: 'Why is my withdrawal pending?', answer: 'It is being processed and confirmed by the exchange. Status updates appear in your activity.' },
      { question: 'What happens if it expires or fails?', answer: 'The locked funds are released and the request is marked expired or failed. Start a new withdrawal.' },
      { question: 'Why is there a minimum?', answer: 'The ERC20 minimum is set by KuCoin and applies before the network fee.' },
    ],
  },
}

export const GUIDE_ORDER = ['deposit', 'buy-contract', 'sell-contract', 'withdraw'] as const
