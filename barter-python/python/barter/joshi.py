import numpy as np
from abc import ABC, abstractmethod
from dataclasses import dataclass
import math


class PayOff(ABC):
    """Abstract payoff function."""
    @abstractmethod
    def __call__(self, spot: float) -> float:
        ...

@dataclass
class Call(PayOff):
    strike: float
    def __call__(self, spot: float) -> float:
        return max(spot - self.strike, 0.0)

@dataclass
class Put(PayOff):
    strike: float
    def __call__(self, spot: float) -> float:
        return max(self.strike - spot, 0.0)

@dataclass
class DigitalCall(PayOff):
    strike: float
    def __call__(self, spot: float) -> float:
        return 1.0 if spot > self.strike else 0.0

@dataclass
class DigitalPut(PayOff):
    strike: float
    def __call__(self, spot: float) -> float:
        return 1.0 if spot < self.strike else 0.0

@dataclass
class DoubleDigital(PayOff):
    lower: float
    upper: float
    def __call__(self, spot: float) -> float:
        return 1.0 if self.lower < spot < self.upper else 0.0

@dataclass
class Straddle(PayOff):
    strike: float
    def __call__(self, spot: float) -> float:
        return abs(spot - self.strike)

def simple_monte_carlo(payoff, spot, rate, vol, expiry, num_paths, seed=42):
    """Simple Monte Carlo European option pricer."""
    rng = np.random.default_rng(seed)
    drift = (rate - 0.5 * vol**2) * expiry
    vol_sqrt_t = vol * math.sqrt(expiry)
    discount = math.exp(-rate * expiry)
    
    z = rng.standard_normal(num_paths)
    spot_t = spot * np.exp(drift + vol_sqrt_t * z)
    payoffs = np.array([payoff(s) for s in spot_t])
    
    mean = payoffs.mean()
    std_error = payoffs.std(ddof=1) / math.sqrt(num_paths)
    
    price = discount * mean
    se = discount * std_error
    return price, se

def monte_carlo_convergence(payoff, spot, rate, vol, expiry, max_paths, seed=42):
    """MC with convergence table at powers of 2."""
    rng = np.random.default_rng(seed)
    drift = (rate - 0.5 * vol**2) * expiry
    vol_sqrt_t = vol * math.sqrt(expiry)
    discount = math.exp(-rate * expiry)
    
    results = []
    running_sum = 0.0
    running_sum_sq = 0.0
    next_record = 2
    
    for i in range(1, max_paths + 1):
        z = rng.standard_normal()
        spot_t = spot * math.exp(drift + vol_sqrt_t * z)
        pv = payoff(spot_t)
        running_sum += pv
        running_sum_sq += pv * pv
        
        if i == next_record:
            mean = running_sum / i
            var = (running_sum_sq / i - mean**2) * i / (i - 1)
            se = math.sqrt(var / i)
            results.append((i, discount * mean, discount * se))
            next_record *= 2
    
    return results

def monte_carlo_antithetic(payoff, spot, rate, vol, expiry, num_paths, seed=42):
    rng = np.random.default_rng(seed)
    drift = (rate - 0.5 * vol**2) * expiry
    vol_sqrt_t = vol * math.sqrt(expiry)
    discount = math.exp(-rate * expiry)
    
    half = num_paths // 2
    z = rng.standard_normal(half)
    
    spot_up = spot * np.exp(drift + vol_sqrt_t * z)
    spot_down = spot * np.exp(drift - vol_sqrt_t * z)
    
    payoffs = 0.5 * (np.array([payoff(s) for s in spot_up]) + np.array([payoff(s) for s in spot_down]))
    
    mean = payoffs.mean()
    se = payoffs.std(ddof=1) / math.sqrt(half)
    return discount * mean, discount * se

def normal_cdf(x):
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))

def normal_pdf(x):
    return math.exp(-0.5 * x * x) / math.sqrt(2.0 * math.pi)

def d1_d2(S, K, r, sigma, T):
    vol_sqrt_t = sigma * math.sqrt(T)
    d1 = (math.log(S / K) + (r + 0.5 * sigma**2) * T) / vol_sqrt_t
    d2 = d1 - vol_sqrt_t
    return d1, d2

def bs_call(S, K, r, sigma, T):
    d1, d2 = d1_d2(S, K, r, sigma, T)
    return S * normal_cdf(d1) - K * math.exp(-r * T) * normal_cdf(d2)

def bs_put(S, K, r, sigma, T):
    d1, d2 = d1_d2(S, K, r, sigma, T)
    return K * math.exp(-r * T) * normal_cdf(-d2) - S * normal_cdf(-d1)

def call_delta(S, K, r, sigma, T):
    d1, _ = d1_d2(S, K, r, sigma, T)
    return normal_cdf(d1)

def gamma(S, K, r, sigma, T):
    d1, _ = d1_d2(S, K, r, sigma, T)
    return normal_pdf(d1) / (S * sigma * math.sqrt(T))

def vega(S, K, r, sigma, T):
    d1, _ = d1_d2(S, K, r, sigma, T)
    return S * normal_pdf(d1) * math.sqrt(T)

def call_theta(S, K, r, sigma, T):
    d1, d2 = d1_d2(S, K, r, sigma, T)
    t1 = -S * normal_pdf(d1) * sigma / (2 * math.sqrt(T))
    t2 = -r * K * math.exp(-r * T) * normal_cdf(d2)
    return t1 + t2

def call_rho(S, K, r, sigma, T):
    _, d2 = d1_d2(S, K, r, sigma, T)
    return K * T * math.exp(-r * T) * normal_cdf(d2)

def binomial_european(payoff_fn, S, r, sigma, T, steps):
    dt = T / steps
    u = math.exp(sigma * math.sqrt(dt))
    d = 1.0 / u
    disc = math.exp(-r * dt)
    p = (math.exp(r * dt) - d) / (u - d)
    q = 1.0 - p
    
    # Terminal payoffs
    values = [payoff_fn(S * u**i * d**(steps - i)) for i in range(steps + 1)]
    
    # Backward induction
    for step in range(steps - 1, -1, -1):
        values = [disc * (p * values[i+1] + q * values[i]) for i in range(step + 1)]
    
    return values[0]

def binomial_american(payoff_fn, S, r, sigma, T, steps):
    dt = T / steps
    u = math.exp(sigma * math.sqrt(dt))
    d = 1.0 / u
    disc = math.exp(-r * dt)
    p = (math.exp(r * dt) - d) / (u - d)
    q = 1.0 - p
    
    values = [payoff_fn(S * u**i * d**(steps - i)) for i in range(steps + 1)]
    
    for step in range(steps - 1, -1, -1):
        for i in range(step + 1):
            continuation = disc * (p * values[i+1] + q * values[i])
            exercise = payoff_fn(S * u**i * d**(step - i))
            values[i] = max(continuation, exercise)
    
    return values[0]

def normal_cdf(x):
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))

def normal_pdf(x):
    return math.exp(-0.5 * x * x) / math.sqrt(2 * math.pi)

def bs_call(S, K, r, sigma, T):
    d1 = (math.log(S/K) + (r + 0.5*sigma**2)*T) / (sigma*math.sqrt(T))
    d2 = d1 - sigma*math.sqrt(T)
    return S*normal_cdf(d1) - K*math.exp(-r*T)*normal_cdf(d2)

def bs_vega(S, K, r, sigma, T):
    d1 = (math.log(S/K) + (r + 0.5*sigma**2)*T) / (sigma*math.sqrt(T))
    return S * normal_pdf(d1) * math.sqrt(T)

def implied_vol_newton(market_price, S, K, r, T, initial_vol=0.2, tol=1e-8, max_iter=100):
    vol = initial_vol
    for i in range(max_iter):
        price = bs_call(S, K, r, vol, T)
        v = bs_vega(S, K, r, vol, T)
        diff = price - market_price
        if abs(diff) < tol:
            return vol, i + 1
        if abs(v) < 1e-15:
            raise ValueError("Vega is zero")
        vol -= diff / v
    raise ValueError("Did not converge")

def asian_call_mc(S, K, r, sigma, T, num_dates, num_paths, seed=42):
    rng = np.random.default_rng(seed)
    dt = T / num_dates
    drift = (r - 0.5 * sigma**2) * dt
    vol_sqrt_dt = sigma * math.sqrt(dt)
    discount = math.exp(-r * T)
    
    payoffs = np.zeros(num_paths)
    for p in range(num_paths):
        spot = S
        total = 0.0
        for _ in range(num_dates):
            spot *= math.exp(drift + vol_sqrt_dt * rng.standard_normal())
            total += spot
        avg = total / num_dates
        payoffs[p] = max(avg - K, 0)
    
    price = discount * payoffs.mean()
    se = discount * payoffs.std(ddof=1) / math.sqrt(num_paths)
    return price, se

def barrier_up_out_call_mc(S, K, barrier, r, sigma, T, num_dates, num_paths, seed=42):
    rng = np.random.default_rng(seed)
    dt = T / num_dates
    drift = (r - 0.5 * sigma**2) * dt
    vol_sqrt_dt = sigma * math.sqrt(dt)
    discount = math.exp(-r * T)
    
    payoffs = np.zeros(num_paths)
    for p in range(num_paths):
        spot = S
        knocked_out = False
        for _ in range(num_dates):
            spot *= math.exp(drift + vol_sqrt_dt * rng.standard_normal())
            if spot >= barrier:
                knocked_out = True
                break
        payoffs[p] = 0.0 if knocked_out else max(spot - K, 0)
    
    price = discount * payoffs.mean()
    se = discount * payoffs.std(ddof=1) / math.sqrt(num_paths)
    return price, se

def lookback_call_mc(S, r, sigma, T, num_dates, num_paths, seed=42):
    rng = np.random.default_rng(seed)
    dt = T / num_dates
    drift = (r - 0.5 * sigma**2) * dt
    vol_sqrt_dt = sigma * math.sqrt(dt)
    discount = math.exp(-r * T)
    
    payoffs = np.zeros(num_paths)
    for p in range(num_paths):
        spot = S
        min_spot = S
        for _ in range(num_dates):
            spot *= math.exp(drift + vol_sqrt_dt * rng.standard_normal())
            min_spot = min(min_spot, spot)
        payoffs[p] = spot - min_spot
    
    price = discount * payoffs.mean()
    se = discount * payoffs.std(ddof=1) / math.sqrt(num_paths)
    return price, se

