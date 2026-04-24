# Dokumentation Gig4

# Inventory
Inventories ägs och hanteras av sina entities. Entities kommunicerar med sina inventories genom meddelanden. Tanken är att ett inventory endast kommunicerar med den ägande etitien, och att alla förfrågningar till andra actors ska hanteras av entityn.

## Användning

```pub fn init() -> AID<InventoryMessage>``` skapar och kör en ny inventory actor. Funktionen returnerar det nya inventoryts AID. För att få inventoryt att göra saker skickar man meddelanden till inventoryts AID med InventoryMessage-enumet.

## Meddelanden
InventoryMessage-enumet är uppdelat i två delar. Den första ska användas av den ägande entityn, och den andra av andra inventories.

Följande meddelanden kan användas av ägaren för att kommunicera med inventories. **Dessa bör endast skickas av den ägande etityn**:

* ### ```Add(AID<EntityMessage>, (Item, usize))```

    Ber inventoryt att lägga till något antal av items. Skickar alltid tillbaka ett InventoryOk-meddelande till ägaren, då inventories i dagsläget inte har någon maxgräns på antal items. Detta meddelande är tänkt att användas av byggnader för att producera items.

* ### ```Remove(AID<EntityMessage>, (Item, usize))```

    Ber inventoryt ta bort något antal items. Skickar ett InventoryOk till ägaren om inventoryt lyckades ta bort den önskade mängden items, annars skickas ett InventoryErr. Detta meddelande är tänkt att användas av byggnarer för att antingen bara förstöra items, eller i produktionsprocessen där man konverterar ett eller flera items till ett nytt item.

* ### ```TakeFrom(AID<EntityMessage>, AID<InventoryMessage>, (Item, usize))```

    Ber inventoryt att ta något antal items från ett annat inventory. Detta meddelande är tänkt att användas av workers när de ska ta items från byggnader.

    Notera: Detta meddelande kräver flera meddelandeutbyten mellan inventories. Alltså hanteras inte hela förfrågningen på en gång, utan båda inventory-aktörerna måste dela privata meddelanden med varandra för att synkronisera flytten.

* ### ```GiveTo(AID<EntityMessage>, AID<InventoryMessage>, (Item, usize))```

    Ber inventoryt ge något antal items till ett annat inventory. Detta meddelande är tänkt att användas av workers när de ska lämna över items till byggnader. 

    Notera: Detta meddelande kräver flera meddelandeutbyten mellan inventories. Alltså hanteras inte hela förfrågningen på en gång, utan båda inventory-aktörerna måste dela privata meddelanden med varandra för att synkronisera flytten.

* ### ```PrintInventory(String)```

    Ber inventoryt skriva ut allt innehåll. Meddelandet tar in en sträng som ska vara namnet på inventoryt så att man kan utskilja olika inventory printouts från varandra. Detta meddelande är i dagsläget endast tänk för debuggning.

* ### ```Kill```

    Ber inventoryt att sluta. I dagsläget avslutar inventory sin loop och struntar i alla andra meddelanden i sin mailbox. 

Följande meddelanden är endast till för kommunikation mellan två inventories. **De ska endast skickas av ett inventory**. Man kan tänka att dessa är privata.

* ### ```GiveMeItems(AID<EntityMessage>, AID<InventoryMessage>, (Item, usize))```

    Ber ett annat inventory kolla om det kan ge items till detta inventoryt. Detta meddelande skickas i samband med ett TakeFrom meddelande. Efter ett GiveMeItems-meddelande förväntar detta inventory få ett GiveMeItemsResult-meddelande av det andra inventoryt.

* ### ```GiveMeItemResult(AID<EntityMessage>, Result<(Item, usize), &'static str>)```

    Svarar på ett GiveMeItems-meddelande genom att skicka ett Result som antingen innehåller de items som har flyttats, eller en sträng som innehåller vad som gick fel. 

* ### ```TakeMyItems(AID<EntityMessage>, AID<InventoryMessage>, (Item, usize))```

    Ber ett annat inventory kolla om det kan ta emot items från detta inventoryt. Detta meddelande skickas i samband med ett GiveTo meddelande. Efter ett TakeMyItems-meddelande förväntar detta inventory få ett TakeMyItems-meddelande av det andra inventoryt.

* ### ```TakeMyItemsResult(AID<EntityMessage>, Result<(Item, usize), &'static str>)```

    Svarar på ett TakeMzyItems-meddelande genom att skicka ett Result som antingen inneehåller vilka items den kolla att den kan ta emot, eller en sträng som innehåller vad som gick fel.

## AID
AIDs (Actor Identifiers) är den grundläggande datastrukturen som ska användas för alla actors. 

``` rust
pub struct AID<T> {
    tid: thread::ThreadId,
    channel: mpsc::Sender<T>,
}
```

En AID innehåller en ThreadId och sender-delen av en mpsc-kanal (multi-producer, single consumer-kanal).

## Entity
## Task
## WorldManager

