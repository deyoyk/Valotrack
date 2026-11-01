use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Value};

use crate::{
    app::AppState,
    content::{self, ItemIndex, CURRENCY_KC, CURRENCY_RAD, CURRENCY_VP},
};

fn resolve_reward(index: &ItemIndex, owned: &std::collections::HashSet<String>, reward: &Value) -> Value {
    let item_type = reward
        .get("ItemTypeID")
        .and_then(Value::as_str)
        .unwrap_or("");
    let item_id = reward.get("ItemID").and_then(Value::as_str).unwrap_or("");
    let item_type = if [
        content::CURRENCY_VP,
        content::CURRENCY_KC,
        content::CURRENCY_RAD,
    ]
    .contains(&item_id)
    {
        item_id
    } else {
        item_type
    };
    let mut item = index.resolve(item_type, item_id);
    item["owned"] = json!(content::is_owned(index, owned, item_type, item_id));
    item["amount"] = reward.get("Quantity").cloned().unwrap_or(json!(1));
    item
}

fn price_for(
    offer: &Value,
    price_map: &HashMap<String, i64>,
    override_cost: Option<i64>,
) -> (i64, &'static str) {
    if let Some(cost) = override_cost {
        return (cost, "vp");
    }
    let direct = content::currency_cost(offer.get("Cost").unwrap_or(&Value::Null));
    if direct.0 > 0 {
        return direct;
    }
    let fallback = offer
        .get("OfferID")
        .and_then(Value::as_str)
        .and_then(|id| price_map.get(id).copied())
        .unwrap_or(0);
    (fallback, "vp")
}

fn resolve_offer(
    index: &ItemIndex,
    owned: &std::collections::HashSet<String>,
    offer: &Value,
    discount: i64,
    override_cost: Option<i64>,
    price_map: &HashMap<String, i64>,
) -> Value {
    let rewards = offer
        .get("Rewards")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut item = rewards
        .first()
        .map(|reward| resolve_reward(index, owned, reward))
        .unwrap_or_else(|| {
            json!({
                "type": "item",
                "name": "Item",
                "icon": null,
                "owned": false
            })
        });
    item["offerId"] = offer.get("OfferID").cloned().unwrap_or(Value::Null);
    let (cost, currency) = price_for(offer, price_map, override_cost);
    item["cost"] = json!(cost);
    item["currency"] = json!(currency);
    item["discountPercent"] = json!(discount);
    if rewards.len() > 1 {
        item["rewards"] = json!(rewards
            .iter()
            .map(|reward| resolve_reward(index, owned, reward))
            .collect::<Vec<_>>());
    }
    item
}

pub async fn snapshot(state: &Arc<AppState>, refresh: bool) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("store:{}", session.puuid);
    if !refresh {
        if let Some(cached) = state.cache_json(&cache_key) {
            return Ok(cached);
        }
    }

    let storefront = match session
        .pd_post(
            &format!("/store/v3/storefront/{}", session.puuid),
            Some(&json!({})),
        )
        .await
    {
        Ok(value) if value.get("SkinsPanelLayout").is_some() => value,
        _ => session
            .pd_get(&format!("/store/v2/storefront/{}", session.puuid))
            .await
            .map_err(|error| format!("Storefront unavailable: {error}"))?,
    };
    let price_map = session
        .pd_get("/store/v1/offers/")
        .await
        .ok()
        .and_then(|value| value.get("Offers").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|offers| {
            offers
                .into_iter()
                .filter_map(|offer| {
                    let id = offer.get("OfferID").and_then(Value::as_str)?;
                    let cost = content::vp_cost(offer.get("Cost").unwrap_or(&Value::Null));
                    Some((id.to_string(), cost))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let wallet = session
        .pd_get(&format!("/store/v1/wallet/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| value.get("Balances").cloned())
        .unwrap_or_else(|| json!({}));
    let index = ItemIndex::build(&state.valo).await;
    let owned = content::owned_set(&session, &index).await;
    let account = content::account_label(&session).await;

    let layout = storefront
        .get("SkinsPanelLayout")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let single_offers = layout
        .get("SingleItemStoreOffers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let offer_by_id: HashMap<String, Value> = single_offers
        .iter()
        .filter_map(|offer| {
            let id = offer.get("OfferID").and_then(Value::as_str)?;
            Some((id.to_string(), offer.clone()))
        })
        .collect();
    let daily = layout
        .get("SingleItemOffers")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .filter_map(|id| offer_by_id.get(id))
                .map(|offer| resolve_offer(&index, &owned, offer, 0, None, &price_map))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let featured = storefront
        .get("FeaturedBundle")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut bundle_rows = featured
        .get("Bundles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if bundle_rows.is_empty() {
        if let Some(bundle) = featured.get("Bundle").filter(|value| value.is_object()) {
            bundle_rows.push(bundle.clone());
        }
    }
    let bundles = bundle_rows
        .iter()
        .map(|bundle| {
            let meta = index.bundle(
                bundle
                    .get("DataAssetID")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            );
            let items = bundle
                .get("Items")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|entry| {
                            let reward = entry.get("Item").cloned().unwrap_or_else(|| json!({}));
                            let mut item = resolve_reward(&index, &owned, &reward);
                            item["basePrice"] = entry.get("BasePrice").cloned().unwrap_or(json!(0));
                            item["discountedPrice"] = entry
                                .get("DiscountedPrice")
                                .cloned()
                                .unwrap_or(json!(0));
                            item["discountPercent"] = entry
                                .get("DiscountPercent")
                                .cloned()
                                .unwrap_or(json!(0));
                            item["isPromoItem"] = entry
                                .get("IsPromoItem")
                                .cloned()
                                .unwrap_or(json!(false));
                            item
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "id": bundle.get("ID").cloned().unwrap_or(Value::Null),
                "dataAssetId": bundle.get("DataAssetID").cloned().unwrap_or(Value::Null),
                "name": meta.get("name").cloned().unwrap_or(json!("Bundle")),
                "icon": meta.get("icon").cloned().unwrap_or(Value::Null),
                "items": items,
                "baseCost": content::vp_cost(bundle.get("TotalBaseCost").unwrap_or(&Value::Null)),
                "discountedCost": content::vp_cost(bundle.get("TotalDiscountedCost").unwrap_or(&Value::Null)),
                "discountPercent": bundle.get("TotalDiscountPercent").and_then(Value::as_i64).unwrap_or(0),
                "remaining": bundle.get("DurationRemainingInSeconds").and_then(Value::as_i64).unwrap_or(0),
                "wholesaleOnly": bundle.get("WholesaleOnly").and_then(Value::as_bool).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();

    let night_market = storefront
        .get("BonusStore")
        .filter(|value| value.is_object())
        .map(|bonus| {
            let offers = bonus
                .get("BonusStoreOffers")
                .and_then(Value::as_array)
                .map(|offers| {
                    offers
                        .iter()
                        .filter_map(|entry| {
                            let offer = entry.get("Offer")?;
                            let cost = entry
                                .get("DiscountCosts")
                                .map(content::vp_cost)
                                .filter(|cost| *cost > 0);
                            Some(resolve_offer(
                                &index,
                                &owned,
                                offer,
                                entry
                                    .get("DiscountPercent")
                                    .and_then(Value::as_i64)
                                    .unwrap_or(0),
                                cost,
                                &price_map,
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "remaining": bonus.get("BonusStoreRemainingDurationInSeconds").and_then(Value::as_i64).unwrap_or(0),
                "offers": offers
            })
        });

    let accessories = storefront
        .get("AccessoryStore")
        .and_then(|store| store.get("AccessoryStoreOffers"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("Offer"))
                .map(|offer| resolve_offer(&index, &owned, offer, 0, None, &price_map))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let upgrade_currency = storefront
        .get("UpgradeCurrencyStore")
        .and_then(|store| store.get("UpgradeCurrencyOffers"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let offer = row.get("Offer")?;
                    let mut item = resolve_offer(
                        &index,
                        &owned,
                        offer,
                        row.get("DiscountedPercent")
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        None,
                        &price_map,
                    );
                    if let Some(store_id) = row.get("StorefrontItemID").and_then(Value::as_str) {
                        item["storefrontItemId"] = json!(store_id);
                        let resolved = index.resolve(content::ITEM_SKIN, store_id);
                        if resolved.get("name").and_then(Value::as_str) != Some("Item") {
                            item["item"] = resolved;
                        }
                    }
                    Some(item)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let value = json!({
        "available": true,
        "account": account,
        "wallet": {
            "vp": wallet.get(CURRENCY_VP).and_then(Value::as_i64).unwrap_or(0),
            "rad": wallet.get(CURRENCY_RAD).and_then(Value::as_i64).unwrap_or(0),
            "kc": wallet.get(CURRENCY_KC).and_then(Value::as_i64).unwrap_or(0)
        },
        "daily": {
            "remaining": layout.get("SingleItemOffersRemainingDurationInSeconds").and_then(Value::as_i64).unwrap_or(0),
            "offers": daily
        },
        "featured": {
            "remaining": featured.get("BundleRemainingDurationInSeconds").and_then(Value::as_i64).unwrap_or(0),
            "bundles": bundles
        },
        "nightMarket": night_market,
        "accessories": accessories,
        "upgradeCurrency": upgrade_currency,
        "fetchedAt": crate::history::now()
    });
    state.put_json(cache_key, value.clone());
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_prices_prefer_the_offer_currency() {
        let empty = HashMap::new();
        let vp = json!({ "OfferID": "vp", "Cost": { CURRENCY_VP: 2175 } });
        assert_eq!(price_for(&vp, &empty, None), (2175, "vp"));
        let kc = json!({ "OfferID": "kc", "Cost": { CURRENCY_KC: 5500 } });
        assert_eq!(price_for(&kc, &empty, None), (5500, "kc"));
    }

    #[test]
    fn store_prices_fall_back_to_the_price_table_and_override() {
        let mut prices = HashMap::new();
        prices.insert("offer-1".to_string(), 1775);
        let offer = json!({ "OfferID": "offer-1" });
        assert_eq!(price_for(&offer, &prices, None), (1775, "vp"));
        assert_eq!(price_for(&offer, &prices, Some(999)), (999, "vp"));
    }
}
